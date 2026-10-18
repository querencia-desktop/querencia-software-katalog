//! Textnormalisierung und Gewichtung für die Katalogsuche.
//!
//! Gewichte (pro Suchwort, Summe über alle Wörter): Name-Präfix 64 > Name enthält 32 >
//! Schlüsselwort 16 > Zusammenfassung 8 > Paketname 4 > Beschreibung 2. Ein Suchwort muss in
//! mindestens einem Feld vorkommen (alle Wörter müssen treffen). Stimmt der gesamte Suchtext mit
//! dem Namen überein, kommen 128 dazu. Vergleich in Kleinschreibung mit aufgelösten Umlauten
//! (`ä` → `ae`, `ß` → `ss`) und ohne Akzente (`é` → `e`), damit „Buro“, „Büro“ und „Buero“
//! dasselbe finden: Die Hauptsuche vergleicht die Umschrift (`ae`), findet die Hauptsuche nichts,
//! wiederholt eine Rückfallsuche mit `ae`/`oe`/`ue` → `a`/`o`/`u` auf beiden Seiten.

/// Normalisiert Text für den Vergleich: Kleinschreibung, Umlaute → Umschrift, Akzente entfernt,
/// Leerraum auf einzelne Leerzeichen reduziert.
pub fn normalisieren(text: &str) -> String {
    let mut aus = String::with_capacity(text.len() + 8);
    let mut letztes_leer = true;
    for c in text.chars() {
        if c.is_whitespace() {
            if !letztes_leer {
                aus.push(' ');
                letztes_leer = true;
            }
            continue;
        }
        letztes_leer = false;
        for k in c.to_lowercase() {
            match k {
                'ä' => aus.push_str("ae"),
                'ö' => aus.push_str("oe"),
                'ü' => aus.push_str("ue"),
                'ß' => aus.push_str("ss"),
                'æ' => aus.push_str("ae"),
                'œ' => aus.push_str("oe"),
                'ø' => aus.push('o'),
                'à' | 'á' | 'â' | 'ã' | 'å' | 'ā' => aus.push('a'),
                'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' => aus.push('e'),
                'ì' | 'í' | 'î' | 'ï' | 'ī' => aus.push('i'),
                'ò' | 'ó' | 'ô' | 'õ' | 'ō' => aus.push('o'),
                'ù' | 'ú' | 'û' | 'ū' => aus.push('u'),
                'ý' | 'ÿ' => aus.push('y'),
                'ç' | 'č' | 'ć' => aus.push('c'),
                'ñ' | 'ń' | 'ň' => aus.push('n'),
                'š' | 'ś' => aus.push('s'),
                'ž' | 'ź' | 'ż' => aus.push('z'),
                'ł' => aus.push('l'),
                'ř' => aus.push('r'),
                'ď' | 'đ' => aus.push('d'),
                'ť' => aus.push('t'),
                _ => aus.push(k),
            }
        }
    }
    if aus.ends_with(' ') {
        aus.pop();
    }
    aus
}

/// Faltet die Umschrift weiter: `ae`/`oe`/`ue` → `a`/`o`/`u`. Damit findet die Rückfallsuche auch
/// „buro“ für „Büro“ (Tastatur ohne Umlaute, ohne Umschrift getippt).
pub fn falten(normalisiert: &str) -> String {
    normalisiert.replace("ae", "a").replace("oe", "o").replace("ue", "u")
}

/// Vorbereitete Suchfelder einer Anwendung (alle normalisiert).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Suchfelder {
    pub name: String,
    pub schluesselwoerter: Vec<String>,
    pub zusammenfassung: String,
    pub paketname: String,
    pub beschreibung: String,
}

impl Suchfelder {
    /// Gefaltete Kopie für die Rückfallsuche (siehe [`falten`]).
    pub fn gefaltet(&self) -> Suchfelder {
        Suchfelder {
            name: falten(&self.name),
            schluesselwoerter: self.schluesselwoerter.iter().map(|k| falten(k)).collect(),
            zusammenfassung: falten(&self.zusammenfassung),
            paketname: falten(&self.paketname),
            beschreibung: falten(&self.beschreibung),
        }
    }
}

pub const GEWICHT_NAME_PRAEFIX: u32 = 64;
pub const GEWICHT_NAME_ENTHAELT: u32 = 32;
pub const GEWICHT_SCHLUESSELWORT: u32 = 16;
pub const GEWICHT_ZUSAMMENFASSUNG: u32 = 8;
pub const GEWICHT_PAKETNAME: u32 = 4;
pub const GEWICHT_BESCHREIBUNG: u32 = 2;
pub const GEWICHT_NAME_GENAU: u32 = 128;

/// Bewertet eine Anwendung gegen einen bereits normalisierten Suchtext. `None` = kein Treffer.
pub fn bewerten(felder: &Suchfelder, suchtext_normalisiert: &str) -> Option<u32> {
    let woerter: Vec<&str> = suchtext_normalisiert.split(' ').filter(|w| !w.is_empty()).collect();
    if woerter.is_empty() {
        return None;
    }
    let mut summe = 0u32;
    for wort in &woerter {
        let mut bestes = 0u32;
        if felder.name.starts_with(wort) || felder.name.split(' ').any(|t| t.starts_with(wort)) {
            bestes = GEWICHT_NAME_PRAEFIX;
        } else if felder.name.contains(wort) {
            bestes = GEWICHT_NAME_ENTHAELT;
        }
        if bestes < GEWICHT_SCHLUESSELWORT
            && felder.schluesselwoerter.iter().any(|k| k.starts_with(wort) || k.contains(wort))
        {
            bestes = GEWICHT_SCHLUESSELWORT;
        }
        if bestes < GEWICHT_ZUSAMMENFASSUNG && felder.zusammenfassung.contains(wort) {
            bestes = GEWICHT_ZUSAMMENFASSUNG;
        }
        if bestes < GEWICHT_PAKETNAME && felder.paketname.contains(wort) {
            bestes = GEWICHT_PAKETNAME;
        }
        if bestes < GEWICHT_BESCHREIBUNG && felder.beschreibung.contains(wort) {
            bestes = GEWICHT_BESCHREIBUNG;
        }
        if bestes == 0 {
            return None;
        }
        summe += bestes;
    }
    if felder.name == suchtext_normalisiert {
        summe += GEWICHT_NAME_GENAU;
    }
    Some(summe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umlaute_und_akzente() {
        assert_eq!(normalisieren("Büro-Übersicht  Straße"), "buero-uebersicht strasse");
        assert_eq!(normalisieren("Éditeur"), "editeur");
        assert_eq!(normalisieren("  GIMP "), "gimp");
    }

    #[test]
    fn faltung() {
        assert_eq!(falten(&normalisieren("Büro")), "buro");
        assert_eq!(falten("buero"), "buro");
    }

    #[test]
    fn gewichtung() {
        let f = Suchfelder {
            name: normalisieren("Texteditor"),
            schluesselwoerter: vec![normalisieren("Editor"), normalisieren("Schreiben")],
            zusammenfassung: normalisieren("Einfacher Editor für Notizen"),
            paketname: "gedit".into(),
            beschreibung: normalisieren("Ein Programm zum Bearbeiten von Texten."),
        };
        assert_eq!(bewerten(&f, "text"), Some(GEWICHT_NAME_PRAEFIX));
        assert_eq!(bewerten(&f, "editor"), Some(GEWICHT_NAME_ENTHAELT));
        assert_eq!(bewerten(&f, "schreib"), Some(GEWICHT_SCHLUESSELWORT));
        assert_eq!(bewerten(&f, "notizen"), Some(GEWICHT_ZUSAMMENFASSUNG));
        assert_eq!(bewerten(&f, "gedit"), Some(GEWICHT_PAKETNAME));
        assert_eq!(bewerten(&f, "bearbeiten"), Some(GEWICHT_BESCHREIBUNG));
        assert_eq!(bewerten(&f, "texteditor"), Some(GEWICHT_NAME_PRAEFIX + GEWICHT_NAME_GENAU));
        assert_eq!(bewerten(&f, "text notizen"), Some(GEWICHT_NAME_PRAEFIX + GEWICHT_ZUSAMMENFASSUNG));
        assert_eq!(bewerten(&f, "text fehltganz"), None);
        assert_eq!(bewerten(&f, ""), None);
    }
}
