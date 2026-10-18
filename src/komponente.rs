//! Quellenunabhängige Rohform einer AppStream-Komponente, wie sie DEP-11- und XML-Leser liefern.

/// Lokalisierte Texte: `(Locale, Text)`. Locales sind normalisiert (`-` → `_`), `C` ist die
/// unübersetzte Fassung. Es werden nur die Locales behalten, die der Katalog braucht
/// (siehe [`locale_behalten`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lokalisiert(pub Vec<(String, String)>);

/// Suchreihenfolge für Texte: Deutsch, Deutsch (Deutschland), unübersetzt, dann Englisch als
/// letzte Rettung für Quellen ohne `C`-Fassung.
pub const LOCALE_REIHENFOLGE: [&str; 6] = ["de", "de_DE", "C", "en", "en_GB", "en_US"];

/// Normalisiert eine Locale-Angabe (`de-DE`, `de_DE.UTF-8`, `de_DE@euro` → `de_DE`).
pub fn locale_normalisieren(roh: &str) -> String {
    let ohne_zusatz = roh.split(['.', '@']).next().unwrap_or(roh);
    ohne_zusatz.replace('-', "_")
}

/// Ob eine (normalisierte) Locale überhaupt gespeichert wird.
pub fn locale_behalten(locale: &str) -> bool {
    LOCALE_REIHENFOLGE.contains(&locale)
}

impl Lokalisiert {
    /// Fügt einen Text hinzu, falls die Locale behalten wird. Spätere Einträge derselben Locale
    /// ersetzen frühere.
    pub fn setzen(&mut self, locale: &str, text: String) {
        let locale = locale_normalisieren(locale);
        if !locale_behalten(&locale) {
            return;
        }
        if let Some(e) = self.0.iter_mut().find(|(l, _)| *l == locale) {
            e.1 = text;
        } else {
            self.0.push((locale, text));
        }
    }

    /// Text zu einer bestimmten Locale.
    pub fn genau(&self, locale: &str) -> Option<&str> {
        self.0.iter().find(|(l, _)| l == locale).map(|(_, t)| t.as_str())
    }

    /// Bester Text nach [`LOCALE_REIHENFOLGE`]; sonst der erste vorhandene.
    pub fn bester(&self) -> Option<&str> {
        LOCALE_REIHENFOLGE
            .iter()
            .find_map(|l| self.genau(l))
            .or_else(|| self.0.first().map(|(_, t)| t.as_str()))
    }

    /// Ob eine deutsche Fassung vorhanden ist (de oder de_DE, nicht leer).
    pub fn deutsch(&self) -> bool {
        self.genau("de").or_else(|| self.genau("de_DE")).is_some_and(|t| !t.trim().is_empty())
    }

    pub fn ist_leer(&self) -> bool {
        self.0.is_empty()
    }
}

/// Lokalisierte Schlüsselwortlisten: `(Locale, Wörter)`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schluesselwoerter(pub Vec<(String, Vec<String>)>);

impl Schluesselwoerter {
    pub fn setzen(&mut self, locale: &str, woerter: Vec<String>) {
        let locale = locale_normalisieren(locale);
        if !locale_behalten(&locale) || woerter.is_empty() {
            return;
        }
        if let Some(e) = self.0.iter_mut().find(|(l, _)| *l == locale) {
            e.1 = woerter;
        } else {
            self.0.push((locale, woerter));
        }
    }

    /// Schlüsselwörter zum Suchen: alle behaltenen Fassungen vereinigt, in
    /// [`LOCALE_REIHENFOLGE`] (Deutsch zuerst), ohne Dubletten.
    pub fn alle_fuer_suche(&self) -> Vec<String> {
        let mut aus = Vec::new();
        for locale in LOCALE_REIHENFOLGE {
            let Some((_, w)) = self.0.iter().find(|(l, _)| l == locale) else { continue };
            for wort in w {
                if !aus.iter().any(|a: &String| a.eq_ignore_ascii_case(wort)) {
                    aus.push(wort.clone());
                }
            }
        }
        aus
    }
}

/// Symbol im Katalog-Cache (`cached`): Datei unter `<icons>/<origin>/<breite>x<hoehe>[@skala]/`.
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolDatei {
    pub name: String,
    pub breite: u32,
    pub hoehe: u32,
    pub skala: u32,
}

/// Symbol auf dem Medienserver (`remote`): wird offline nicht geladen, aber gemerkt.
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolFern {
    /// Vollständige URL (MediaBaseUrl bereits vorangestellt).
    pub url: String,
    pub breite: u32,
    pub hoehe: u32,
}

/// Rohe Komponente, noch ohne Kategorienabbildung und Textumwandlung.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RohKomponente {
    /// AppStream-Typ (`desktop-application`, `console-application`, `addon`, …).
    /// Leer in MetaInfo-Dateien ohne `type`-Attribut (entspricht `generic`).
    pub typ: String,
    pub id: String,
    pub paketname: Option<String>,
    /// Komponente wird als Snap/Flatpak/… ausgeliefert (`Bundles:` bzw. `<bundle>`).
    pub bundle: bool,
    pub name: Lokalisiert,
    pub zusammenfassung: Lokalisiert,
    /// Beschreibung noch mit AppStream-Markup (`p`, `ul`, `ol`, `li`, `em`, `code`).
    pub beschreibung: Lokalisiert,
    pub schluesselwoerter: Schluesselwoerter,
    /// Freedesktop-Kategorien, roh.
    pub kategorien: Vec<String>,
    /// Themensymbol (`stock`).
    pub symbol_thema: Option<String>,
    pub symbole_cache: Vec<SymbolDatei>,
    pub symbole_fern: Vec<SymbolFern>,
    /// Lokale Symboldatei (`local`, nur MetaInfo/XML; absoluter Pfad).
    pub symbol_lokal: Option<String>,
    /// Bildschirmfoto-URLs (Quellbild bevorzugt, sonst größtes Vorschaubild).
    pub bildschirmfotos: Vec<String>,
    pub lizenz: Option<String>,
    pub homepage: Option<String>,
    /// Origin der Quelle (`ubuntu-resolute-universe`); leer bei MetaInfo.
    pub origin: String,
}

impl RohKomponente {
    /// Ob die Komponente eine Anwendung im Sinne des Katalogs ist (Typfilter, noch ohne Paketprüfung).
    pub fn ist_anwendungstyp(&self) -> bool {
        matches!(self.typ.as_str(), "desktop-application" | "console-application")
    }
}

/// Kopfdaten einer Katalogquelle (Metadatenheader).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Quellkopf {
    pub origin: String,
    pub media_base_url: Option<String>,
}

impl Quellkopf {
    /// Macht eine relative Medien-URL absolut.
    pub fn medien_url(&self, relativ: &str) -> String {
        if relativ.contains("://") {
            return relativ.to_string();
        }
        match &self.media_base_url {
            Some(b) => format!("{}/{}", b.trim_end_matches('/'), relativ.trim_start_matches('/')),
            None => relativ.to_string(),
        }
    }
}
