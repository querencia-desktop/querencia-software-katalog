//! Minimalparser für die YAML-Teilmenge, die `appstream-generator` in DEP-11-Dateien schreibt.
//!
//! Unterstützt: Block-Abbildungen und Block-Listen (auch `Schlüssel:` gefolgt von `- ` auf gleicher
//! Einrückung), einfache und mehrzeilige Skalare (plain, `'…'`, `"…"` mit Escapes und
//! Zeilenfaltung), Blockskalare `|` und `>` mit Chomping (`-`, `+`), leere Flusssammlungen `{}`
//! und `[]`, flache Flusslisten `[a, b]`, Dokumenttrenner `---`/`...`, Kommentarzeilen und
//! in Anführungszeichen gesetzte Schlüssel (`'no':`).
//!
//! Explizite Schlüssel (`? Schlüssel` / `: Wert`, einzeilig) kommen in Ubuntus Daten elfmal vor
//! (fehlerhaft gedrehte Übersetzungen) und werden gelesen.
//!
//! Nicht unterstützt (kommt in DEP-11 nicht vor und führt zu einem Fehler): Anker/Aliasse,
//! Tags, verschachtelte Flusssammlungen, mehrzeilige explizite Schlüssel. Ein `#` innerhalb
//! einer Zeile gilt **nicht** als Kommentar; ganze Kommentarzeilen werden außerhalb von
//! Blockskalaren übersprungen.

use std::fmt;

/// Ein YAML-Knoten.
#[derive(Clone, Debug, PartialEq)]
pub enum Knoten {
    Null,
    Skalar(String),
    Liste(Vec<Knoten>),
    Abbildung(Vec<(String, Knoten)>),
}

impl Knoten {
    /// Wert zu einem Schlüssel einer Abbildung.
    pub fn feld(&self, name: &str) -> Option<&Knoten> {
        match self {
            Knoten::Abbildung(e) => e.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Skalartext, falls der Knoten ein Skalar ist.
    pub fn text(&self) -> Option<&str> {
        match self {
            Knoten::Skalar(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Listenelemente (leer, wenn kein Listenknoten).
    pub fn liste(&self) -> &[Knoten] {
        match self {
            Knoten::Liste(l) => l,
            _ => &[],
        }
    }

    /// Einträge einer Abbildung (leer, wenn keine Abbildung).
    pub fn eintraege(&self) -> &[(String, Knoten)] {
        match self {
            Knoten::Abbildung(e) => e,
            _ => &[],
        }
    }

    /// Text eines Feldes, falls vorhanden und Skalar.
    pub fn feld_text(&self, name: &str) -> Option<&str> {
        self.feld(name).and_then(Knoten::text)
    }
}

/// Fehler beim Einlesen; nennt die Zeile (1-basiert).
#[derive(Clone, Debug, PartialEq)]
pub struct YamlFehler {
    pub zeile: usize,
    pub meldung: String,
}

impl fmt::Display for YamlFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "YAML Zeile {}: {}", self.zeile, self.meldung)
    }
}

impl std::error::Error for YamlFehler {}

#[derive(Clone, Copy, Debug)]
struct Zeile<'a> {
    nummer: usize,
    einzug: usize,
    /// Inhalt ohne führende Leerzeichen und ohne `\r`.
    inhalt: &'a str,
}

impl Zeile<'_> {
    fn leer(&self) -> bool {
        self.inhalt.is_empty()
    }

    /// Ganze Kommentarzeile; zählt strukturell wie eine Leerzeile, ist aber in Blockskalaren Inhalt.
    fn kommentar(&self) -> bool {
        self.inhalt.starts_with('#')
    }

    fn ueberspringbar(&self) -> bool {
        self.leer() || self.kommentar()
    }
}

struct Parser<'a> {
    zeilen: Vec<Zeile<'a>>,
    pos: usize,
}

/// Liest alle Dokumente eines YAML-Textes.
pub fn dokumente(text: &str) -> Result<Vec<Knoten>, YamlFehler> {
    let mut zeilen = Vec::with_capacity(text.len() / 40 + 1);
    for (i, roh) in text.split('\n').enumerate() {
        let roh = roh.strip_suffix('\r').unwrap_or(roh);
        let inhalt = roh.trim_start_matches(' ');
        let einzug = roh.len() - inhalt.len();
        let inhalt = if inhalt.trim().is_empty() { "" } else { inhalt };
        zeilen.push(Zeile {
            nummer: i + 1,
            einzug,
            inhalt,
        });
    }
    let mut p = Parser { zeilen, pos: 0 };
    let mut docs = Vec::new();
    loop {
        p.ueberspringe_leere();
        let Some(z) = p.aktuell() else { break };
        if z.einzug == 0 && (z.inhalt == "---" || z.inhalt.starts_with("--- ")) {
            p.pos += 1;
            p.ueberspringe_leere();
            match p.aktuell() {
                None => docs.push(Knoten::Null),
                Some(n) if n.einzug == 0 && (n.inhalt == "---" || n.inhalt == "...") => {
                    docs.push(Knoten::Null)
                }
                Some(_) => docs.push(p.block()?),
            }
        } else if z.einzug == 0 && z.inhalt == "..." {
            p.pos += 1;
        } else {
            docs.push(p.block()?);
        }
    }
    Ok(docs)
}

fn ist_listenpunkt(inhalt: &str) -> bool {
    inhalt == "-" || inhalt.starts_with("- ")
}

fn ist_trenner(z: &Zeile<'_>) -> bool {
    z.einzug == 0 && (z.inhalt == "---" || z.inhalt == "..." || z.inhalt.starts_with("--- "))
}

impl<'a> Parser<'a> {
    fn aktuell(&self) -> Option<Zeile<'a>> {
        self.zeilen.get(self.pos).copied()
    }

    fn ueberspringe_leere(&mut self) {
        while matches!(self.aktuell(), Some(z) if z.ueberspringbar()) {
            self.pos += 1;
        }
    }

    /// Nächste nicht leere Zeile (ohne den Cursor zu bewegen), kein Dokumenttrenner.
    fn naechste_inhaltszeile(&self) -> Option<Zeile<'a>> {
        self.zeilen[self.pos..]
            .iter()
            .find(|z| !z.ueberspringbar())
            .copied()
            .filter(|z| !ist_trenner(z))
    }

    fn fehler(&self, meldung: impl Into<String>) -> YamlFehler {
        YamlFehler {
            zeile: self.aktuell().map(|z| z.nummer).unwrap_or(0),
            meldung: meldung.into(),
        }
    }

    /// Block auf der Einrückung der aktuellen (nicht leeren) Zeile.
    fn block(&mut self) -> Result<Knoten, YamlFehler> {
        self.ueberspringe_leere();
        let z = self
            .aktuell()
            .ok_or_else(|| self.fehler("unerwartetes Ende"))?;
        if ist_listenpunkt(z.inhalt) {
            self.liste(z.einzug)
        } else {
            self.abbildung(z.einzug)
        }
    }

    fn liste(&mut self, einzug: usize) -> Result<Knoten, YamlFehler> {
        let mut elemente = Vec::new();
        loop {
            self.ueberspringe_leere();
            let Some(z) = self.aktuell() else { break };
            if ist_trenner(&z) || z.einzug != einzug || !ist_listenpunkt(z.inhalt) {
                break;
            }
            let rest = z.inhalt[1..].trim_start_matches(' ');
            if rest.is_empty() {
                self.pos += 1;
                match self.naechste_inhaltszeile() {
                    Some(n) if n.einzug > einzug => elemente.push(self.block()?),
                    _ => elemente.push(Knoten::Null),
                }
            } else if ist_abbildungsstart(rest) {
                // Abbildung, die in der Listenzeile beginnt: Zeile virtuell umschreiben.
                let spalte = einzug + (z.inhalt.len() - rest.len());
                self.zeilen[self.pos] = Zeile {
                    nummer: z.nummer,
                    einzug: spalte,
                    inhalt: rest,
                };
                elemente.push(self.abbildung(spalte)?);
            } else {
                let spalte = einzug + (z.inhalt.len() - rest.len());
                self.zeilen[self.pos] = Zeile {
                    nummer: z.nummer,
                    einzug: spalte,
                    inhalt: rest,
                };
                elemente.push(self.wert(rest, einzug)?);
            }
        }
        Ok(Knoten::Liste(elemente))
    }

    fn abbildung(&mut self, einzug: usize) -> Result<Knoten, YamlFehler> {
        let mut eintraege = Vec::new();
        loop {
            self.ueberspringe_leere();
            let Some(z) = self.aktuell() else { break };
            if ist_trenner(&z) || z.einzug != einzug || ist_listenpunkt(z.inhalt) {
                break;
            }
            if let Some(expl) = z.inhalt.strip_prefix("? ") {
                // Expliziter Schlüssel: `? Schlüssel` und in der nächsten Zeile `: Wert`.
                let schluessel = entzitiere_einfach(expl.trim());
                self.pos += 1;
                self.ueberspringe_leere();
                let wert = match self.aktuell() {
                    Some(w)
                        if w.einzug == einzug
                            && (w.inhalt == ":" || w.inhalt.starts_with(": ")) =>
                    {
                        let rest = w.inhalt[1..].trim();
                        if rest.is_empty() {
                            self.pos += 1;
                            match self.naechste_inhaltszeile() {
                                Some(n) if n.einzug > einzug => self.block()?,
                                _ => Knoten::Null,
                            }
                        } else {
                            self.wert(rest, einzug)?
                        }
                    }
                    _ => Knoten::Null,
                };
                eintraege.push((schluessel, wert));
                continue;
            }
            let (schluessel, rest) = self.schluessel(z.inhalt)?;
            let rest = rest.trim();
            let wert = if rest.is_empty() {
                self.pos += 1;
                match self.naechste_inhaltszeile() {
                    Some(n) if n.einzug > einzug => self.block()?,
                    Some(n) if n.einzug == einzug && ist_listenpunkt(n.inhalt) => {
                        self.liste(einzug)?
                    }
                    _ => Knoten::Null,
                }
            } else {
                self.wert(rest, einzug)?
            };
            eintraege.push((schluessel, wert));
        }
        Ok(Knoten::Abbildung(eintraege))
    }

    /// Zerlegt `schlüssel: rest`. Der Rückgabewert `rest` ist der Text nach dem Doppelpunkt.
    fn schluessel(&self, inhalt: &'a str) -> Result<(String, &'a str), YamlFehler> {
        if let Some(q) = inhalt.chars().next().filter(|c| *c == '\'' || *c == '"') {
            let mut ende = None;
            let mut iter = inhalt.char_indices().skip(1).peekable();
            while let Some((i, c)) = iter.next() {
                if c == q {
                    if q == '\'' && iter.peek().map(|(_, n)| *n) == Some('\'') {
                        iter.next();
                        continue;
                    }
                    ende = Some(i);
                    break;
                }
                if c == '\\' && q == '"' {
                    iter.next();
                }
            }
            let ende = ende
                .ok_or_else(|| self.fehler("Anführungszeichen im Schlüssel nicht geschlossen"))?;
            let roh = &inhalt[1..ende];
            let danach = inhalt[ende + 1..].trim_start();
            let rest = danach
                .strip_prefix(':')
                .ok_or_else(|| self.fehler("Doppelpunkt nach Schlüssel erwartet"))?;
            let key = if q == '\'' {
                roh.replace("''", "'")
            } else {
                entkomme_doppelt(roh)
            };
            return Ok((key, rest));
        }
        // Plain-Schlüssel: erstes ": " oder ":" am Zeilenende.
        let bytes = inhalt.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b':' && (i + 1 == bytes.len() || bytes[i + 1] == b' ') {
                return Ok((inhalt[..i].trim_end().to_string(), &inhalt[i + 1..]));
            }
            i += 1;
        }
        Err(self.fehler(format!(
            "Abbildungseintrag erwartet, gefunden: {:?}",
            inhalt
        )))
    }

    /// Wert, der in der aktuellen Zeile (nach Schlüssel oder `- `) beginnt. `einzug` ist die
    /// Einrückung des Elternknotens; Fortsetzungszeilen müssen tiefer eingerückt sein.
    fn wert(&mut self, rest: &'a str, einzug: usize) -> Result<Knoten, YamlFehler> {
        let erstes = rest.chars().next().unwrap_or(' ');
        match erstes {
            '|' | '>' => self.blockskalar(rest, einzug),
            '\'' | '"' => {
                let s = self.zitiert(rest, erstes)?;
                Ok(Knoten::Skalar(s))
            }
            '{' => {
                if rest.trim() == "{}" {
                    self.pos += 1;
                    Ok(Knoten::Abbildung(Vec::new()))
                } else {
                    Err(self.fehler("Flussabbildung mit Inhalt wird nicht unterstützt"))
                }
            }
            '[' => {
                let inner = rest.trim();
                let inner = inner
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .ok_or_else(|| self.fehler("Flussliste nicht abgeschlossen"))?;
                self.pos += 1;
                let elemente = inner
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| Knoten::Skalar(entzitiere_einfach(s)))
                    .collect();
                Ok(Knoten::Liste(elemente))
            }
            '&' | '*' | '!' => Err(self.fehler("Anker, Aliasse und Tags werden nicht unterstützt")),
            _ => {
                // Plain-Skalar, ggf. mit Fortsetzungszeilen.
                let mut s = String::from(rest.trim_end());
                self.pos += 1;
                let mut leere = 0usize;
                while let Some(z) = self.aktuell() {
                    if z.leer() {
                        leere += 1;
                        self.pos += 1;
                        continue;
                    }
                    if ist_trenner(&z) || z.einzug <= einzug || z.kommentar() {
                        break;
                    }
                    if leere == 0 {
                        s.push(' ');
                    } else {
                        for _ in 0..leere {
                            s.push('\n');
                        }
                        leere = 0;
                    }
                    s.push_str(z.inhalt.trim_end());
                    self.pos += 1;
                }
                // Übersprungene Leerzeilen am Ende gehören nicht zum Skalar; Cursor zurück.
                self.pos -= leere;
                Ok(skalar_oder_null(s))
            }
        }
    }

    fn blockskalar(&mut self, kopf: &str, einzug: usize) -> Result<Knoten, YamlFehler> {
        let gefaltet = kopf.starts_with('>');
        let kopf = kopf.trim();
        let chomp = if kopf.contains('-') {
            '-'
        } else if kopf.contains('+') {
            '+'
        } else {
            ' '
        };
        if kopf.len() > 3
            || kopf
                .chars()
                .skip(1)
                .any(|c| !matches!(c, '-' | '+' | '1'..='9'))
        {
            return Err(self.fehler(format!("Blockskalar-Kopf nicht unterstützt: {:?}", kopf)));
        }
        self.pos += 1;
        // Blockeinzug = Einzug der ersten nicht leeren Zeile (muss > einzug sein).
        let mut blockeinzug = None;
        // None = Leerzeile, sonst (zusätzlicher Einzug, Text).
        let mut zeilen: Vec<Option<(usize, &'a str)>> = Vec::new();
        while let Some(z) = self.aktuell() {
            if z.leer() {
                zeilen.push(None);
                self.pos += 1;
                continue;
            }
            if ist_trenner(&z) || z.einzug <= einzug {
                break;
            }
            let be = *blockeinzug.get_or_insert(z.einzug);
            if z.einzug < be {
                break;
            }
            // Zusätzlicher Einzug über dem Blockeinzug bleibt erhalten.
            zeilen.push(Some((z.einzug - be, z.inhalt)));
            self.pos += 1;
        }
        // Leerzeilen am Ende abtrennen (gehören zum Chomping).
        let mut nachlauf = 0;
        while matches!(zeilen.last(), Some(None)) {
            zeilen.pop();
            nachlauf += 1;
        }
        self.pos -= nachlauf; // zurück: diese Leerzeilen trennen ggf. den nächsten Eintrag
        let mut s = String::new();
        if gefaltet {
            let mut leere = 0usize;
            let mut vorher_eingerueckt = false;
            let mut erste = true;
            for z in &zeilen {
                match z {
                    None => leere += 1,
                    Some((extra, t)) => {
                        let eingerueckt = *extra > 0 || t.starts_with('\t');
                        if !erste {
                            if leere > 0 {
                                for _ in 0..leere {
                                    s.push('\n');
                                }
                                if eingerueckt || vorher_eingerueckt {
                                    s.push('\n');
                                }
                            } else if eingerueckt || vorher_eingerueckt {
                                s.push('\n');
                            } else {
                                s.push(' ');
                            }
                        }
                        s.extend(std::iter::repeat_n(' ', *extra));
                        s.push_str(t);
                        leere = 0;
                        vorher_eingerueckt = eingerueckt;
                        erste = false;
                    }
                }
            }
        } else {
            let mut erste = true;
            for z in &zeilen {
                if !erste {
                    s.push('\n');
                }
                erste = false;
                if let Some((extra, t)) = z {
                    s.extend(std::iter::repeat_n(' ', *extra));
                    s.push_str(t);
                }
            }
        }
        match chomp {
            '-' => {}
            '+' => {
                s.push('\n');
                for _ in 0..nachlauf {
                    s.push('\n');
                }
            }
            _ => s.push('\n'),
        }
        Ok(Knoten::Skalar(s))
    }

    /// Zitierter Skalar, ggf. über mehrere Zeilen. `rest` beginnt mit dem Anführungszeichen.
    fn zitiert(&mut self, rest: &'a str, q: char) -> Result<String, YamlFehler> {
        let mut ergebnis = String::new();
        let mut zeile: &str = &rest[1..];
        let mut erste_zeile = true;
        loop {
            // Zeile durchsuchen.
            let mut chars = zeile.char_indices().peekable();
            let mut geschlossen = None;
            let mut fortsetzung_ohne_leerzeichen = false;
            while let Some((i, c)) = chars.next() {
                if c == q {
                    if q == '\'' && chars.peek().map(|(_, n)| *n) == Some('\'') {
                        ergebnis.push('\'');
                        chars.next();
                        continue;
                    }
                    geschlossen = Some(i);
                    break;
                }
                if c == '\\' && q == '"' {
                    match chars.next() {
                        None => {
                            // Escaped Zeilenumbruch: ohne Leerzeichen fortsetzen.
                            fortsetzung_ohne_leerzeichen = true;
                            break;
                        }
                        Some((_, e)) => match e {
                            'n' => ergebnis.push('\n'),
                            't' => ergebnis.push('\t'),
                            'r' => ergebnis.push('\r'),
                            '0' => ergebnis.push('\0'),
                            'a' => ergebnis.push('\u{7}'),
                            'b' => ergebnis.push('\u{8}'),
                            'e' => ergebnis.push('\u{1b}'),
                            'f' => ergebnis.push('\u{c}'),
                            'v' => ergebnis.push('\u{b}'),
                            ' ' => ergebnis.push(' '),
                            '_' => ergebnis.push('\u{a0}'),
                            'N' => ergebnis.push('\u{85}'),
                            'L' => ergebnis.push('\u{2028}'),
                            'P' => ergebnis.push('\u{2029}'),
                            'x' | 'u' | 'U' => {
                                let n = match e {
                                    'x' => 2,
                                    'u' => 4,
                                    _ => 8,
                                };
                                let mut hex = String::new();
                                for _ in 0..n {
                                    if let Some((_, h)) = chars.next() {
                                        hex.push(h);
                                    }
                                }
                                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                                    Some(ch) => ergebnis.push(ch),
                                    None => {
                                        return Err(self
                                            .fehler(format!("ungültiges Escape \\{}{}", e, hex)))
                                    }
                                }
                            }
                            andere => ergebnis.push(andere),
                        },
                    }
                    continue;
                }
                ergebnis.push(c);
            }
            if geschlossen.is_some() {
                self.pos += 1;
                return Ok(ergebnis);
            }
            // Zeilenende ohne schließendes Anführungszeichen → Faltung.
            let len = ergebnis.trim_end_matches([' ', '\t']).len();
            ergebnis.truncate(len);
            self.pos += 1;
            let mut leere = 0;
            loop {
                match self.aktuell() {
                    None => return Err(self.fehler("Anführungszeichen nicht geschlossen")),
                    Some(z) if z.leer() => {
                        leere += 1;
                        self.pos += 1;
                    }
                    Some(z) => {
                        if ist_trenner(&z) && !erste_zeile {
                            return Err(self.fehler("Anführungszeichen nicht geschlossen"));
                        }
                        zeile = z.inhalt;
                        break;
                    }
                }
            }
            erste_zeile = false;
            if !fortsetzung_ohne_leerzeichen {
                if leere == 0 {
                    ergebnis.push(' ');
                } else {
                    for _ in 0..leere {
                        ergebnis.push('\n');
                    }
                }
            }
        }
    }
}

fn ist_abbildungsstart(rest: &str) -> bool {
    let erstes = rest.chars().next().unwrap_or(' ');
    if matches!(erstes, '\'' | '"') {
        // Zitierter Schlüssel: nach dem schließenden Zeichen folgt ':'.
        let mut iter = rest.char_indices().skip(1).peekable();
        while let Some((i, c)) = iter.next() {
            if c == erstes {
                if erstes == '\'' && iter.peek().map(|(_, n)| *n) == Some('\'') {
                    iter.next();
                    continue;
                }
                return rest[i + 1..].trim_start().starts_with(':');
            }
            if c == '\\' && erstes == '"' {
                iter.next();
            }
        }
        return false;
    }
    if matches!(erstes, '[' | '{' | '|' | '>' | '&' | '*' | '!') {
        return false;
    }
    let b = rest.as_bytes();
    (0..b.len()).any(|i| b[i] == b':' && (i + 1 == b.len() || b[i + 1] == b' '))
}

fn skalar_oder_null(s: String) -> Knoten {
    if s == "~" || s == "null" {
        Knoten::Null
    } else {
        Knoten::Skalar(s)
    }
}

fn entzitiere_einfach(s: &str) -> String {
    if let Some(inner) = s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        inner.replace("''", "'")
    } else if let Some(inner) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        entkomme_doppelt(inner)
    } else {
        s.to_string()
    }
}

fn entkomme_doppelt(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eins(text: &str) -> Knoten {
        let d = dokumente(text).expect("parsen");
        assert_eq!(d.len(), 1, "genau ein Dokument erwartet: {:?}", d);
        d.into_iter().next().unwrap()
    }

    #[test]
    fn abbildung_und_liste() {
        let k =
            eins("Type: desktop-application\nCategories:\n- A\n- B\nName:\n  C: Foo\n  de: Fuu\n");
        assert_eq!(k.feld_text("Type"), Some("desktop-application"));
        let kat: Vec<_> = k
            .feld("Categories")
            .unwrap()
            .liste()
            .iter()
            .map(|n| n.text().unwrap())
            .collect();
        assert_eq!(kat, ["A", "B"]);
        assert_eq!(k.feld("Name").unwrap().feld_text("de"), Some("Fuu"));
    }

    #[test]
    fn liste_mit_abbildungen() {
        let k = eins("Icon:\n  cached:\n  - name: a.png\n    width: 64\n    height: 64\n  - name: b.png\n    width: 48\n  stock: foo\n");
        let icon = k.feld("Icon").unwrap();
        let cached = icon.feld("cached").unwrap().liste();
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].feld_text("name"), Some("a.png"));
        assert_eq!(cached[0].feld_text("width"), Some("64"));
        assert_eq!(cached[1].feld_text("name"), Some("b.png"));
        assert_eq!(icon.feld_text("stock"), Some("foo"));
    }

    #[test]
    fn gefalteter_blockskalar() {
        let k = eins("Description:\n  C: >-\n    <p>Erste Zeile\n    zweite Zeile.</p>\n\n    <p>Neuer Absatz.</p>\n  de: >-\n    <p>Deutsch</p>\nNext: 1\n");
        let d = k.feld("Description").unwrap();
        assert_eq!(
            d.feld_text("C"),
            Some("<p>Erste Zeile zweite Zeile.</p>\n<p>Neuer Absatz.</p>")
        );
        assert_eq!(d.feld_text("de"), Some("<p>Deutsch</p>"));
        assert_eq!(k.feld_text("Next"), Some("1"));
    }

    #[test]
    fn literaler_blockskalar_mit_clip() {
        let k = eins("a: |\n  x\n  y\nb: 2\n");
        assert_eq!(k.feld_text("a"), Some("x\ny\n"));
        assert_eq!(k.feld_text("b"), Some("2"));
    }

    #[test]
    fn zitierte_skalare() {
        let k = eins("a: 'Puzzle ''x'' y: z'\nb: \"Zeile\\n\\tTab \\U0001F600 \\\"q\\\"\"\nc: \"mehr\n  zeilig\"\n'no': Norsk\n");
        assert_eq!(k.feld_text("a"), Some("Puzzle 'x' y: z"));
        assert_eq!(k.feld_text("b"), Some("Zeile\n\tTab 😀 \"q\""));
        assert_eq!(k.feld_text("c"), Some("mehr zeilig"));
        assert_eq!(k.feld_text("no"), Some("Norsk"));
    }

    #[test]
    fn plain_mehrzeilig_und_leere_fluss() {
        let k = eins("a: erste\n  zweite\nb: {}\nc: []\nd:\ne: [x, 'y']\n");
        assert_eq!(k.feld_text("a"), Some("erste zweite"));
        assert_eq!(k.feld("b"), Some(&Knoten::Abbildung(vec![])));
        assert_eq!(k.feld("c"), Some(&Knoten::Liste(vec![])));
        assert_eq!(k.feld("d"), Some(&Knoten::Null));
        assert_eq!(k.feld("e").unwrap().liste().len(), 2);
    }

    #[test]
    fn mehrere_dokumente() {
        let d =
            dokumente("---\nFile: DEP-11\nOrigin: x\n---\nType: a\nID: 1\n---\nType: b\nID: 2\n")
                .unwrap();
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].feld_text("Origin"), Some("x"));
        assert_eq!(d[2].feld_text("ID"), Some("2"));
    }

    #[test]
    fn expliziter_schluessel_und_kommentare() {
        let k = eins("# Kommentar\nName:\n  C: Foo\n  ? Langer Schlüssel\n  : ne_NP\n  # noch einer\n  de: Fu\nD: >-\n  Text\n  # kein Kommentar\n  weiter\n");
        let n = k.feld("Name").unwrap();
        assert_eq!(n.feld_text("Langer Schlüssel"), Some("ne_NP"));
        assert_eq!(n.feld_text("de"), Some("Fu"));
        assert_eq!(k.feld_text("D"), Some("Text # kein Kommentar weiter"));
    }

    #[test]
    fn fehler_bei_anker() {
        assert!(dokumente("a: &x b\n").is_err());
    }

    #[test]
    fn verschachtelte_liste_in_liste_mit_abbildung() {
        let k = eins("Screenshots:\n- default: true\n  thumbnails:\n  - url: a\n    width: 1\n  source-image:\n    url: b\n- caption: {}\n  source-image:\n    url: c\n");
        let s = k.feld("Screenshots").unwrap().liste();
        assert_eq!(s.len(), 2);
        assert_eq!(
            s[0].feld("source-image").unwrap().feld_text("url"),
            Some("b")
        );
        assert_eq!(
            s[0].feld("thumbnails").unwrap().liste()[0].feld_text("url"),
            Some("a")
        );
        assert_eq!(
            s[1].feld("source-image").unwrap().feld_text("url"),
            Some("c")
        );
    }
}

#[cfg(test)]
mod tests_einzug {
    use super::*;

    #[test]
    fn mehr_eingerueckte_zeile_in_faltung() {
        let d = dokumente("a: >-\n  x\n    y\n  z\n").unwrap();
        assert_eq!(d[0].feld_text("a"), Some("x\n  y\nz"));
    }
}
