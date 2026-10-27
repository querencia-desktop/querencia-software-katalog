//! Leser für AppStream-XML: Katalogdateien (`<components>`) und MetaInfo-Dateien (`<component>`).

use std::path::Path;

use roxmltree::{Document, Node};

use crate::komponente::{
    Lokalisiert, Quellkopf, RohKomponente, Schluesselwoerter, SymbolDatei, SymbolFern,
};

/// Liest eine XML-Datei (`.xml` oder `.xml.gz`).
pub fn lese_datei(pfad: &Path) -> Result<(Quellkopf, Vec<RohKomponente>), String> {
    let roh = std::fs::read(pfad).map_err(|e| format!("{}: {}", pfad.display(), e))?;
    let text = if pfad.extension().is_some_and(|e| e == "gz") || roh.starts_with(&[0x1f, 0x8b]) {
        use std::io::Read;
        let mut s = String::new();
        flate2::read::MultiGzDecoder::new(&roh[..])
            .read_to_string(&mut s)
            .map_err(|e| format!("{}: gzip: {}", pfad.display(), e))?;
        s
    } else {
        String::from_utf8(roh).map_err(|e| format!("{}: {}", pfad.display(), e))?
    };
    lese_text(&text).map_err(|e| format!("{}: {}", pfad.display(), e))
}

/// Liest AppStream-XML aus einem Text.
pub fn lese_text(text: &str) -> Result<(Quellkopf, Vec<RohKomponente>), String> {
    // `appstreamcli convert` schreibt für leere DEP-11-Quellen eine leere Datei.
    if text.trim().is_empty() {
        return Ok((Quellkopf::default(), Vec::new()));
    }
    let doc = Document::parse(text).map_err(|e| format!("XML: {}", e))?;
    let wurzel = doc.root_element();
    let mut kopf = Quellkopf::default();
    let mut komponenten = Vec::new();
    match wurzel.tag_name().name() {
        "components" => {
            kopf.origin = wurzel.attribute("origin").unwrap_or("").to_string();
            kopf.media_base_url = wurzel.attribute("media_baseurl").map(str::to_string);
            for c in wurzel
                .children()
                .filter(|c| c.is_element() && c.tag_name().name() == "component")
            {
                if let Some(k) = komponente(c, &kopf) {
                    komponenten.push(k);
                }
            }
        }
        "component" => {
            if let Some(k) = komponente(wurzel, &kopf) {
                komponenten.push(k);
            }
        }
        andere => return Err(format!("XML: unerwartetes Wurzelelement <{}>", andere)),
    }
    Ok((kopf, komponenten))
}

fn lang<'a>(n: Node<'a, 'a>) -> &'a str {
    n.attribute(("http://www.w3.org/XML/1998/namespace", "lang"))
        .unwrap_or("C")
}

fn text(n: Node<'_, '_>) -> String {
    n.text().unwrap_or("").trim().to_string()
}

fn kinder<'a>(n: Node<'a, 'a>, name: &'a str) -> impl Iterator<Item = Node<'a, 'a>> {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn zahl(n: Node, attr: &str) -> u32 {
    n.attribute(attr)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0)
}

/// Serialisiert den Inhalt eines Elements wieder als Markup (für die gemeinsame Textumwandlung).
fn innen_markup(n: Node, aus: &mut String) {
    for c in n.children() {
        if c.is_text() {
            aus.push_str(&escape(c.text().unwrap_or("")));
        } else if c.is_element() {
            // Fremdsprachige Elemente mitten in einer Fassung gehören nicht in deren Text.
            if c.has_attribute(("http://www.w3.org/XML/1998/namespace", "lang")) {
                continue;
            }
            let name = c.tag_name().name();
            aus.push('<');
            aus.push_str(name);
            aus.push('>');
            innen_markup(c, aus);
            aus.push_str("</");
            aus.push_str(name);
            aus.push('>');
        }
    }
}

fn escape(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Komponententyp; `desktop` ist die Altschreibweise von `desktop-application` (AppStream),
/// die installierte MetaInfo-Dateien noch nutzen (z. B. GIMP).
fn typ_normalisieren(typ: &str) -> String {
    match typ.trim() {
        "desktop" => "desktop-application".to_string(),
        t => t.to_string(),
    }
}

fn komponente(c: Node, kopf: &Quellkopf) -> Option<RohKomponente> {
    let id = kinder(c, "id").next().map(text).filter(|s| !s.is_empty())?;
    let mut k = RohKomponente {
        typ: typ_normalisieren(c.attribute("type").unwrap_or("generic")),
        id,
        paketname: kinder(c, "pkgname")
            .next()
            .map(text)
            .filter(|s| !s.is_empty()),
        bundle: kinder(c, "bundle").next().is_some(),
        lizenz: kinder(c, "project_license")
            .next()
            .map(text)
            .filter(|s| !s.is_empty()),
        homepage: kinder(c, "url")
            .find(|u| u.attribute("type") == Some("homepage"))
            .map(text)
            .filter(|s| !s.is_empty()),
        origin: kopf.origin.clone(),
        ..RohKomponente::default()
    };
    for n in kinder(c, "name") {
        k.name.setzen(lang(n), text(n));
    }
    for n in kinder(c, "summary") {
        k.zusammenfassung.setzen(lang(n), text(n));
    }
    // Beschreibung: Katalog-XML lokalisiert das ganze Element, MetaInfo jeden Absatz einzeln.
    for d in kinder(c, "description") {
        let l = lang(d);
        if l != "C"
            || !d.children().any(|p| {
                p.is_element() && p.has_attribute(("http://www.w3.org/XML/1998/namespace", "lang"))
            })
        {
            let mut m = String::new();
            innen_markup(d, &mut m);
            k.beschreibung.setzen(l, m);
        } else {
            let mut je_locale = Lokalisiert::default();
            let mut sammel: Vec<(String, String)> = Vec::new();
            for p in d.children().filter(|p| p.is_element()) {
                let pl = crate::komponente::locale_normalisieren(lang(p));
                let name = p.tag_name().name();
                let mut m = String::new();
                m.push('<');
                m.push_str(name);
                m.push('>');
                innen_markup(p, &mut m);
                m.push_str("</");
                m.push_str(name);
                m.push('>');
                match sammel.iter_mut().find(|(sl, _)| *sl == pl) {
                    Some(e) => e.1.push_str(&m),
                    None => sammel.push((pl, m)),
                }
            }
            for (sl, m) in sammel {
                je_locale.setzen(&sl, m);
            }
            // Listen ohne eigene Sprachangabe gehören zur C-Fassung; übersetzte Fassungen, die
            // nur Absätze haben, bleiben so wie sie sind (AppStream-Regel).
            for (sl, m) in je_locale.0 {
                k.beschreibung.setzen(&sl, m);
            }
        }
    }
    for kw in kinder(c, "keywords") {
        let kl = lang(kw);
        let mut je_locale: Vec<(String, Vec<String>)> = Vec::new();
        for w in kinder(kw, "keyword") {
            let wl = if kl != "C" {
                kl.to_string()
            } else {
                lang(w).to_string()
            };
            let wort = text(w);
            if wort.is_empty() {
                continue;
            }
            match je_locale.iter_mut().find(|(l, _)| *l == wl) {
                Some(e) => e.1.push(wort),
                None => je_locale.push((wl, vec![wort])),
            }
        }
        let mut s = std::mem::take(&mut k.schluesselwoerter);
        for (l, w) in je_locale {
            s.setzen(&l, w);
        }
        k.schluesselwoerter = s;
    }
    if k.schluesselwoerter.0.is_empty() {
        k.schluesselwoerter = Schluesselwoerter::default();
    }
    k.kategorien = kinder(c, "categories")
        .flat_map(|cs| kinder(cs, "category"))
        .map(text)
        .filter(|s| !s.is_empty())
        .collect();
    for icon in kinder(c, "icon") {
        let wert = text(icon);
        match icon.attribute("type") {
            Some("stock") if !wert.is_empty() => k.symbol_thema = Some(wert),
            Some("cached") => k.symbole_cache.push(SymbolDatei {
                name: wert,
                breite: zahl(icon, "width"),
                hoehe: zahl(icon, "height"),
                skala: zahl(icon, "scale").max(1),
            }),
            Some("remote") => k.symbole_fern.push(SymbolFern {
                url: kopf.medien_url(&wert),
                breite: zahl(icon, "width"),
                hoehe: zahl(icon, "height"),
            }),
            Some("local") if !wert.is_empty() => k.symbol_lokal = Some(wert),
            _ => {}
        }
    }
    for foto in kinder(c, "screenshots").flat_map(|s| kinder(s, "screenshot")) {
        let bilder: Vec<Node> = kinder(foto, "image").collect();
        let quelle = bilder
            .iter()
            .find(|b| b.attribute("type").is_none_or(|t| t == "source"))
            .or_else(|| bilder.iter().max_by_key(|b| zahl(**b, "width")));
        if let Some(b) = quelle {
            let url = text(*b);
            if !url.is_empty() {
                k.bildschirmfotos.push(kopf.medien_url(&url));
            }
        }
    }
    Some(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metainfo_mit_absatzweiser_lokalisierung() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>org.example.Bar</id>
  <pkgname>bar</pkgname>
  <name>Bar</name>
  <name xml:lang="de">Bär</name>
  <summary>A bar</summary>
  <description>
    <p>First.</p>
    <p xml:lang="de">Erstens.</p>
    <ul><li>one</li></ul>
  </description>
  <keywords>
    <keyword>bar</keyword>
    <keyword xml:lang="de">Riegel</keyword>
  </keywords>
  <categories><category>Office</category></categories>
  <icon type="stock">bar</icon>
  <url type="homepage">https://bar.example</url>
  <screenshots><screenshot type="default"><image type="source">https://bar.example/s.png</image></screenshot></screenshots>
</component>"#;
        let (_, k) = lese_text(xml).unwrap();
        let k = &k[0];
        assert_eq!(k.typ, "desktop-application");
        assert_eq!(k.paketname.as_deref(), Some("bar"));
        assert_eq!(k.name.genau("de"), Some("Bär"));
        assert_eq!(
            k.beschreibung.genau("C"),
            Some("<p>First.</p><ul><li>one</li></ul>")
        );
        assert_eq!(k.beschreibung.genau("de"), Some("<p>Erstens.</p>"));
        assert_eq!(k.schluesselwoerter.alle_fuer_suche(), ["Riegel", "bar"]);
        assert_eq!(k.symbol_thema.as_deref(), Some("bar"));
        assert_eq!(k.bildschirmfotos, ["https://bar.example/s.png"]);
    }

    #[test]
    fn katalog_xml() {
        let xml = r#"<components version="1.0" origin="test" media_baseurl="https://m.example/">
<component type="desktop-application"><id>a.desktop</id><pkgname>a</pkgname><name>A</name><summary>S</summary>
<description xml:lang="de"><p>Deutsch</p></description><description><p>English</p></description>
<icon type="cached" width="64" height="64">a.png</icon><icon type="remote" width="128" height="128">a/b.png</icon>
<keywords xml:lang="de"><keyword>Alpha</keyword></keywords></component>
<component type="addon"><id>b</id><pkgname>b</pkgname><name>B</name><summary>S</summary><bundle type="snap">b</bundle></component>
</components>"#;
        assert_eq!(lese_text("  \n").unwrap().1.len(), 0);
        let (kopf, k) = lese_text(xml).unwrap();
        assert_eq!(kopf.origin, "test");
        assert_eq!(k.len(), 2);
        assert_eq!(k[0].beschreibung.genau("de"), Some("<p>Deutsch</p>"));
        assert_eq!(k[0].beschreibung.genau("C"), Some("<p>English</p>"));
        assert_eq!(k[0].symbole_fern[0].url, "https://m.example/a/b.png");
        assert_eq!(k[0].schluesselwoerter.alle_fuer_suche(), ["Alpha"]);
        assert!(k[1].bundle);
    }

    #[test]
    fn typ_altschreibweise() {
        let xml = r#"<components><component type="desktop"><id>a</id><name>A</name></component>
<component><id>b</id><name>B</name></component></components>"#;
        let (_, k) = lese_text(xml).unwrap();
        assert_eq!(k[0].typ, "desktop-application");
        assert_eq!(k[1].typ, "generic");
    }
}
