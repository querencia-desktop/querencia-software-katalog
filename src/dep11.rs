//! Leser für DEP-11-Katalogdateien (YAML, meist gzip-komprimiert).

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::komponente::{Lokalisiert, Quellkopf, RohKomponente, Schluesselwoerter, SymbolDatei, SymbolFern};
use crate::yaml::{self, Knoten};

/// Liest eine DEP-11-Datei (`.yml` oder `.yml.gz`).
pub fn lese_datei(pfad: &Path) -> Result<(Quellkopf, Vec<RohKomponente>), String> {
    let mut roh = Vec::new();
    File::open(pfad)
        .and_then(|mut f| f.read_to_end(&mut roh))
        .map_err(|e| format!("{}: {}", pfad.display(), e))?;
    let text = if pfad.extension().is_some_and(|e| e == "gz") || roh.starts_with(&[0x1f, 0x8b]) {
        let mut d = flate2::read::MultiGzDecoder::new(&roh[..]);
        let mut s = String::with_capacity(roh.len() * 4);
        d.read_to_string(&mut s)
            .map_err(|e| format!("{}: gzip: {}", pfad.display(), e))?;
        s
    } else {
        String::from_utf8(roh).map_err(|e| format!("{}: {}", pfad.display(), e))?
    };
    lese_text(&text).map_err(|e| format!("{}: {}", pfad.display(), e))
}

/// Liest DEP-11-YAML aus einem Text.
pub fn lese_text(text: &str) -> Result<(Quellkopf, Vec<RohKomponente>), String> {
    let docs = yaml::dokumente(text).map_err(|e| e.to_string())?;
    let mut kopf = Quellkopf::default();
    let mut komponenten = Vec::with_capacity(docs.len());
    for doc in &docs {
        if doc.feld_text("File") == Some("DEP-11") {
            kopf.origin = doc.feld_text("Origin").unwrap_or("").to_string();
            kopf.media_base_url = doc.feld_text("MediaBaseUrl").map(str::to_string);
            continue;
        }
        if matches!(doc, Knoten::Null) {
            continue;
        }
        if let Some(k) = komponente(doc, &kopf) {
            komponenten.push(k);
        }
    }
    Ok((kopf, komponenten))
}

fn lokalisiert(knoten: Option<&Knoten>) -> Lokalisiert {
    let mut l = Lokalisiert::default();
    if let Some(k) = knoten {
        for (locale, wert) in k.eintraege() {
            if let Some(t) = wert.text() {
                l.setzen(locale, t.trim().to_string());
            }
        }
    }
    l
}

fn zahl(k: &Knoten, name: &str) -> u32 {
    k.feld_text(name).and_then(|t| t.trim().parse().ok()).unwrap_or(0)
}

fn komponente(doc: &Knoten, kopf: &Quellkopf) -> Option<RohKomponente> {
    let id = doc.feld_text("ID")?.trim().to_string();
    let mut k = RohKomponente {
        typ: doc.feld_text("Type").unwrap_or("generic").trim().to_string(),
        id,
        paketname: doc.feld_text("Package").map(|p| p.trim().to_string()).filter(|p| !p.is_empty()),
        bundle: doc.feld("Bundles").is_some_and(|b| !b.liste().is_empty() || !b.eintraege().is_empty()),
        name: lokalisiert(doc.feld("Name")),
        zusammenfassung: lokalisiert(doc.feld("Summary")),
        beschreibung: lokalisiert(doc.feld("Description")),
        lizenz: doc.feld_text("ProjectLicense").map(|s| s.trim().to_string()),
        homepage: doc.feld("Url").and_then(|u| u.feld_text("homepage")).map(|s| s.trim().to_string()),
        origin: kopf.origin.clone(),
        ..RohKomponente::default()
    };
    if let Some(kw) = doc.feld("Keywords") {
        let mut s = Schluesselwoerter::default();
        for (locale, liste) in kw.eintraege() {
            let woerter: Vec<String> = liste
                .liste()
                .iter()
                .filter_map(Knoten::text)
                .map(|w| w.trim().to_string())
                .filter(|w| !w.is_empty())
                .collect();
            s.setzen(locale, woerter);
        }
        k.schluesselwoerter = s;
    }
    k.kategorien = doc
        .feld("Categories")
        .map(|c| c.liste().iter().filter_map(Knoten::text).map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    if let Some(icon) = doc.feld("Icon") {
        k.symbol_thema = icon.feld_text("stock").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        for c in icon.feld("cached").map(Knoten::liste).unwrap_or(&[]) {
            if let Some(name) = c.feld_text("name") {
                k.symbole_cache.push(SymbolDatei {
                    name: name.trim().to_string(),
                    breite: zahl(c, "width"),
                    hoehe: zahl(c, "height"),
                    skala: zahl(c, "scale").max(1),
                });
            }
        }
        for r in icon.feld("remote").map(Knoten::liste).unwrap_or(&[]) {
            if let Some(url) = r.feld_text("url") {
                k.symbole_fern.push(SymbolFern {
                    url: kopf.medien_url(url.trim()),
                    breite: zahl(r, "width"),
                    hoehe: zahl(r, "height"),
                });
            }
        }
        if let Some(l) = icon.feld_text("local") {
            k.symbol_lokal = Some(l.trim().to_string());
        }
    }
    for foto in doc.feld("Screenshots").map(Knoten::liste).unwrap_or(&[]) {
        let quelle = foto
            .feld("source-image")
            .and_then(|s| s.feld_text("url"))
            .or_else(|| {
                foto.feld("thumbnails")
                    .map(Knoten::liste)
                    .unwrap_or(&[])
                    .iter()
                    .max_by_key(|t| zahl(t, "width"))
                    .and_then(|t| t.feld_text("url"))
            });
        if let Some(url) = quelle {
            k.bildschirmfotos.push(kopf.medien_url(url.trim()));
        }
    }
    Some(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEISPIEL: &str = "---\nFile: DEP-11\nVersion: '1.0'\nOrigin: test-origin\nMediaBaseUrl: https://medien.example/x\n---\nType: desktop-application\nID: org.example.Foo.desktop\nPackage: foo\nName:\n  C: Foo\n  de: Fuh\nSummary:\n  C: A foo\n  de: Ein Fuh\nDescription:\n  C: >-\n    <p>Text</p>\nCategories:\n- Utility\n- TextEditor\nKeywords:\n  C:\n  - editor\n  de:\n  - Editor\nIcon:\n  cached:\n  - name: foo_foo.png\n    width: 64\n    height: 64\n  stock: foo\n  remote:\n  - url: f/o/o.png\n    width: 128\n    height: 128\nUrl:\n  homepage: https://foo.example\nProjectLicense: GPL-3.0+\nScreenshots:\n- default: true\n  thumbnails:\n  - url: f/klein.png\n    width: 224\n    height: 100\n  source-image:\n    url: f/orig.png\n    width: 800\n    height: 600\n";

    #[test]
    fn beispiel_lesen() {
        let (kopf, k) = lese_text(BEISPIEL).unwrap();
        assert_eq!(kopf.origin, "test-origin");
        assert_eq!(k.len(), 1);
        let k = &k[0];
        assert_eq!(k.id, "org.example.Foo.desktop");
        assert_eq!(k.paketname.as_deref(), Some("foo"));
        assert_eq!(k.name.genau("de"), Some("Fuh"));
        assert_eq!(k.zusammenfassung.bester(), Some("Ein Fuh"));
        assert_eq!(k.beschreibung.bester(), Some("<p>Text</p>"));
        assert_eq!(k.kategorien, ["Utility", "TextEditor"]);
        assert_eq!(k.schluesselwoerter.alle_fuer_suche(), ["Editor"]);
        assert_eq!(k.symbol_thema.as_deref(), Some("foo"));
        assert_eq!(k.symbole_cache[0].name, "foo_foo.png");
        assert_eq!(k.symbole_cache[0].skala, 1);
        assert_eq!(k.symbole_fern[0].url, "https://medien.example/x/f/o/o.png");
        assert_eq!(k.bildschirmfotos, ["https://medien.example/x/f/orig.png"]);
        assert_eq!(k.homepage.as_deref(), Some("https://foo.example"));
        assert_eq!(k.lizenz.as_deref(), Some("GPL-3.0+"));
        assert_eq!(k.origin, "test-origin");
        assert!(!k.bundle);
    }
}
