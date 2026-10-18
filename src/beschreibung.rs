//! Umwandlung des AppStream-Beschreibungsmarkups in einfachen Text.
//!
//! **Entscheidung (SW-2):** `Anwendung::beschreibung` ist reiner Text ohne Markup. Absätze
//! werden durch eine Leerzeile getrennt, Listenpunkte beginnen mit „• “ (bei `<ol>` mit
//! „1. “, „2. “ …), `<em>` und `<code>` werden auf ihren Text reduziert. Zeilenumbrüche innerhalb
//! eines Absatzes (Faltung im YAML) werden zu Leerzeichen zusammengezogen. Die Oberfläche kann
//! den Text damit direkt in ein `gtk::Label` mit Umbruch setzen; Pango-Markup wäre ein
//! Sicherheitsrisiko (fremde Daten) und bringt in GNOME-2-Optik keinen Mehrwert.

/// Wandelt AppStream-Markup in einfachen Text um.
pub fn markup_zu_text(markup: &str) -> String {
    let mut b = TextBauer::default();
    let mut rest = markup;
    while let Some(pos) = rest.find('<') {
        b.text(&rest[..pos]);
        let nach = &rest[pos + 1..];
        match nach.find('>') {
            None => {
                // Kaputtes Markup: Rest als Text.
                b.text(&rest[pos..]);
                rest = "";
                break;
            }
            Some(ende) => {
                let tag = nach[..ende].trim();
                b.tag(tag);
                rest = &nach[ende + 1..];
            }
        }
    }
    b.text(rest);
    b.fertig()
}

/// Dekodiert die XML-Entitäten, die in AppStream-Texten vorkommen.
pub fn entitaeten_dekodieren(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut aus = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find('&') {
        aus.push_str(&rest[..pos]);
        let nach = &rest[pos + 1..];
        match nach.find(';').filter(|e| *e <= 10) {
            Some(e) => {
                let name = &nach[..e];
                let ersatz = match name {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    "nbsp" => Some('\u{a0}'),
                    n if n.starts_with("#x") || n.starts_with("#X") => {
                        u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32)
                    }
                    n if n.starts_with('#') => n[1..].parse::<u32>().ok().and_then(char::from_u32),
                    _ => None,
                };
                match ersatz {
                    Some(c) => {
                        aus.push(c);
                        rest = &nach[e + 1..];
                    }
                    None => {
                        aus.push('&');
                        rest = nach;
                    }
                }
            }
            None => {
                aus.push('&');
                rest = nach;
            }
        }
    }
    aus.push_str(rest);
    aus
}

/// Baut aus Textstücken und Tags den Ausgabetext.
#[derive(Default)]
pub struct TextBauer {
    /// Abgeschlossene Blöcke: (ist Listenpunkt, Text).
    bloecke: Vec<(bool, String)>,
    /// Aktueller Block (noch nicht abgeschlossen).
    aktuell: String,
    /// Stapel offener Listen: Zähler (`None` = ungeordnet).
    listen: Vec<Option<usize>>,
    /// Ob der aktuelle Block ein Listenpunkt ist.
    in_punkt: bool,
    /// Tiefe innerhalb eines Elements mit eigenem `xml:lang` (fremde Übersetzung mitten in der
    /// Fassung, kommt bei Ubuntu vor); solange > 0 wird Text verworfen.
    fremdsprache: usize,
}

impl TextBauer {
    /// Fügt Text (mit Entitäten) zum aktuellen Block hinzu, Leerraum zusammengezogen.
    pub fn text(&mut self, roh: &str) {
        if roh.is_empty() || self.fremdsprache > 0 {
            return;
        }
        let dekodiert = entitaeten_dekodieren(roh);
        let beginnt_mit_leerraum = dekodiert.starts_with(char::is_whitespace);
        for (i, wort) in dekodiert.split_whitespace().enumerate() {
            let braucht_abstand = i > 0 || beginnt_mit_leerraum;
            if braucht_abstand && !self.aktuell.is_empty() && !self.aktuell.ends_with(' ') {
                self.aktuell.push(' ');
            }
            self.aktuell.push_str(wort);
        }
        if dekodiert.ends_with(char::is_whitespace) && !self.aktuell.is_empty() && !self.aktuell.ends_with(' ') {
            self.aktuell.push(' ');
        }
    }

    /// Verarbeitet einen Tag-Inhalt ohne spitze Klammern, z. B. `p`, `/p`, `ul`, `li`.
    pub fn tag(&mut self, tag: &str) {
        let schliesst = tag.starts_with('/');
        let selbstschliessend = tag.ends_with('/');
        if self.fremdsprache > 0 {
            if schliesst {
                self.fremdsprache -= 1;
            } else if !selbstschliessend {
                self.fremdsprache += 1;
            }
            return;
        }
        if !schliesst && !selbstschliessend && tag.contains("xml:lang") {
            self.fremdsprache = 1;
            return;
        }
        let name = tag
            .trim_start_matches('/')
            .split([' ', '\t', '\n', '/'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        match (name.as_str(), schliesst) {
            ("p", _) => self.block_abschliessen(),
            ("ul", false) => {
                self.block_abschliessen();
                self.listen.push(None);
            }
            ("ol", false) => {
                self.block_abschliessen();
                self.listen.push(Some(0));
            }
            ("ul", true) | ("ol", true) => {
                self.block_abschliessen();
                self.listen.pop();
            }
            ("li", false) => {
                self.block_abschliessen();
                let tiefe = self.listen.len().saturating_sub(1);
                let marke = match self.listen.last_mut() {
                    Some(Some(n)) => {
                        *n += 1;
                        format!("{}. ", n)
                    }
                    _ => "• ".to_string(),
                };
                self.aktuell.extend(std::iter::repeat_n("  ", tiefe));
                self.aktuell.push_str(&marke);
                self.in_punkt = true;
            }
            ("li", true) => self.block_abschliessen(),
            // em, code, unbekannte Tags: nur der Text zählt.
            _ => {}
        }
    }

    fn block_abschliessen(&mut self) {
        let text = self.aktuell.trim_end().to_string();
        let inhalt = text.trim_start();
        let leer = inhalt.is_empty() || inhalt.trim_end_matches(|c: char| c == '•' || c == '.' || c.is_ascii_digit()).is_empty();
        if !leer {
            self.bloecke.push((self.in_punkt, text));
        }
        self.aktuell.clear();
        self.in_punkt = false;
    }

    /// Liefert den fertigen Text.
    pub fn fertig(mut self) -> String {
        self.block_abschliessen();
        let mut aus = String::new();
        let mut vorher_punkt = false;
        for (i, (punkt, text)) in self.bloecke.iter().enumerate() {
            if i > 0 {
                if *punkt && vorher_punkt {
                    aus.push('\n');
                } else {
                    aus.push_str("\n\n");
                }
            }
            aus.push_str(text.trim_start_matches(|c| c == ' ' && !punkt_eingerueckt(text)));
            vorher_punkt = *punkt;
        }
        aus
    }
}

fn punkt_eingerueckt(text: &str) -> bool {
    text.trim_start().starts_with('•') || text.trim_start().chars().next().is_some_and(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absaetze_und_liste() {
        let m = "<p>Erster Absatz\n    gefaltet.</p>\n<p>Zweiter &amp; <em>betont</em>.</p><ul><li>Punkt eins</li><li>Punkt <code>zwei</code></li></ul><p>Schluss</p>";
        assert_eq!(
            markup_zu_text(m),
            "Erster Absatz gefaltet.\n\nZweiter & betont.\n\n• Punkt eins\n• Punkt zwei\n\nSchluss"
        );
    }

    #[test]
    fn geordnete_liste() {
        assert_eq!(markup_zu_text("<ol><li>a</li><li>b</li></ol>"), "1. a\n2. b");
    }

    #[test]
    fn ohne_markup_und_entitaeten() {
        assert_eq!(markup_zu_text("Nur Text &lt;x&gt; &#8211; &quot;q&quot;"), "Nur Text <x> – \"q\"");
        assert_eq!(markup_zu_text("  "), "");
    }

    #[test]
    fn verschachtelte_liste() {
        assert_eq!(markup_zu_text("<ul><li>a<ul><li>b</li></ul></li></ul>"), "• a\n  • b");
    }

    #[test]
    fn fremdsprachige_elemente_und_leerraum() {
        let m = "<p><em>Features:</em><em xml:lang=\"ca\">Característiques:</em><em xml:lang=\"fr\">Fonctions <code>x</code></em></p><ul>\n<li>\n  <em>Eins</em> zwei</li></ul>";
        assert_eq!(markup_zu_text(m), "Features:\n\n• Eins zwei");
    }

    #[test]
    fn kaputtes_markup() {
        assert_eq!(markup_zu_text("<p>offen"), "offen");
        assert_eq!(markup_zu_text("a < b"), "a < b");
    }
}
