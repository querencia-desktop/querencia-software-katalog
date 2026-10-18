//! Abbildung der Freedesktop-Kategorien auf die zehn Querencia-Kategorien.
//!
//! Mehrfachzuordnung ist erlaubt (z. B. `Database` → Büro und Entwicklung). Komponenten, deren
//! Kategorien keinen Treffer ergeben, landen unter „Sonstiges“. Reine Toolkit- und
//! Umgebungsmarken (`GTK`, `Qt`, `KDE`, `GNOME`, `Core`, …) zählen nicht als Kategorie.

use qs_typen::Kategorie;

use Kategorie::*;

/// Die Abbildungstabelle: Freedesktop-Name → Querencia-Kategorien.
///
/// Hauptkategorien der Spezifikation zuerst, danach die Zusatzkategorien, die in Ubuntus Katalog
/// tatsächlich vorkommen. Groß-/Kleinschreibung wird beim Nachschlagen ignoriert.
pub const ABBILDUNG: &[(&str, &[Kategorie])] = &[
    // Hauptkategorien (freedesktop.org Desktop Menu Specification)
    ("AudioVideo", &[Multimedia]),
    ("Audio", &[Multimedia]),
    ("Video", &[Multimedia]),
    ("Development", &[Entwicklung]),
    ("Education", &[Bildung]),
    ("Game", &[Spiele]),
    ("Graphics", &[Grafik]),
    ("Network", &[Internet]),
    ("Office", &[Buero]),
    ("Science", &[Bildung]),
    ("Settings", &[System]),
    ("System", &[System]),
    ("Utility", &[Zubehoer]),
    // Zusatzkategorien: Zubehör
    ("TextEditor", &[Zubehoer]),
    ("Calculator", &[Zubehoer]),
    ("Clock", &[Zubehoer]),
    ("Archiving", &[Zubehoer]),
    ("Compression", &[Zubehoer]),
    ("FileTools", &[Zubehoer]),
    ("Dictionary", &[Zubehoer]),
    ("TextTools", &[Zubehoer]),
    ("Accessibility", &[Zubehoer]),
    ("TerminalEmulator", &[Zubehoer]),
    ("Printing", &[Zubehoer]),
    ("Spirituality", &[Zubehoer]),
    // Büro
    ("WordProcessor", &[Buero]),
    ("Spreadsheet", &[Buero]),
    ("Presentation", &[Buero]),
    ("Calendar", &[Buero]),
    ("ContactManagement", &[Buero]),
    ("Database", &[Buero, Entwicklung]),
    ("Finance", &[Buero]),
    ("ProjectManagement", &[Buero]),
    ("Publishing", &[Buero]),
    ("Chart", &[Buero]),
    ("Flowchart", &[Buero, Grafik]),
    ("Viewer", &[Buero]),
    ("Documentation", &[Buero]),
    ("Economy", &[Buero]),
    // Grafik
    ("2DGraphics", &[Grafik]),
    ("3DGraphics", &[Grafik]),
    ("RasterGraphics", &[Grafik]),
    ("VectorGraphics", &[Grafik]),
    ("Photography", &[Grafik]),
    ("Scanning", &[Grafik]),
    ("OCR", &[Grafik]),
    ("ImageProcessing", &[Grafik]),
    // Internet
    ("WebBrowser", &[Internet]),
    ("Email", &[Internet]),
    ("InstantMessaging", &[Internet]),
    ("Chat", &[Internet]),
    ("IRCClient", &[Internet]),
    ("Feed", &[Internet]),
    ("News", &[Internet]),
    ("FileTransfer", &[Internet]),
    ("P2P", &[Internet]),
    ("RemoteAccess", &[Internet]),
    ("Telephony", &[Internet]),
    ("VideoConference", &[Internet]),
    ("HamRadio", &[Internet]),
    ("Dialup", &[Internet]),
    ("WebDevelopment", &[Internet, Entwicklung]),
    // Multimedia
    ("Player", &[Multimedia]),
    ("Recorder", &[Multimedia]),
    ("Music", &[Multimedia]),
    ("Midi", &[Multimedia]),
    ("Mixer", &[Multimedia]),
    ("Sequencer", &[Multimedia]),
    ("Tuner", &[Multimedia]),
    ("TV", &[Multimedia]),
    ("AudioVideoEditing", &[Multimedia]),
    ("DiscBurning", &[Multimedia]),
    // Spiele
    ("ActionGame", &[Spiele]),
    ("AdventureGame", &[Spiele]),
    ("ArcadeGame", &[Spiele]),
    ("BoardGame", &[Spiele]),
    ("BlocksGame", &[Spiele]),
    ("CardGame", &[Spiele]),
    ("KidsGame", &[Spiele]),
    ("LogicGame", &[Spiele]),
    ("RolePlaying", &[Spiele]),
    ("Shooter", &[Spiele]),
    ("Simulation", &[Spiele]),
    ("SportsGame", &[Spiele]),
    ("StrategyGame", &[Spiele]),
    ("Emulator", &[Spiele]),
    ("Amusement", &[Spiele]),
    // Entwicklung
    ("IDE", &[Entwicklung]),
    ("Debugger", &[Entwicklung]),
    ("GUIDesigner", &[Entwicklung]),
    ("Profiling", &[Entwicklung]),
    ("RevisionControl", &[Entwicklung]),
    ("Translation", &[Entwicklung]),
    ("Building", &[Entwicklung]),
    ("Electronics", &[Entwicklung, Bildung]),
    ("Engineering", &[Entwicklung, Bildung]),
    // Bildung
    ("Math", &[Bildung]),
    ("NumericalAnalysis", &[Bildung]),
    ("Physics", &[Bildung]),
    ("Chemistry", &[Bildung]),
    ("Biology", &[Bildung]),
    ("Astronomy", &[Bildung]),
    ("Geography", &[Bildung]),
    ("Geoscience", &[Bildung]),
    ("Geology", &[Bildung]),
    ("Languages", &[Bildung]),
    ("Literature", &[Bildung]),
    ("History", &[Bildung]),
    ("Art", &[Bildung]),
    ("Construction", &[Bildung]),
    ("ComputerScience", &[Bildung]),
    ("DataVisualization", &[Bildung]),
    ("Humanities", &[Bildung]),
    ("MedicalSoftware", &[Bildung]),
    ("ParallelComputing", &[Bildung]),
    ("Robotics", &[Bildung]),
    ("Sports", &[Bildung]),
    ("ArtificialIntelligence", &[Bildung]),
    ("Maps", &[Bildung]),
    // System
    ("Monitor", &[System]),
    ("Security", &[System]),
    ("Filesystem", &[System]),
    ("FileManager", &[System]),
    ("PackageManager", &[System]),
    ("DesktopSettings", &[System]),
    ("HardwareSettings", &[System]),
    ("Adult", &[Sonstiges]),
];

/// Marken, die keine Kategorie sind (Toolkit, Umgebung); sie lösen kein „Sonstiges“ aus,
/// wenn daneben echte Kategorien stehen, zählen aber auch nicht als Treffer.
pub const KEINE_KATEGORIE: &[&str] = &[
    "GTK", "Qt", "KDE", "GNOME", "XFCE", "MATE", "Cinnamon", "LXDE", "LXQt", "Core", "Motif", "Java",
    "ConsoleOnly", "X-Unknown",
];

/// Querencia-Kategorien zu einem Freedesktop-Namen (leer, wenn unbekannt oder keine Kategorie).
pub fn abbilden(freedesktop: &str) -> &'static [Kategorie] {
    let name = freedesktop.trim();
    for (fd, ziel) in ABBILDUNG {
        if fd.eq_ignore_ascii_case(name) {
            return ziel;
        }
    }
    &[]
}

/// Alle Querencia-Kategorien einer Komponente, in Anzeigereihenfolge, ohne Dubletten; mindestens
/// „Sonstiges“.
pub fn zuordnen<S: AsRef<str>>(freedesktop: &[S]) -> Vec<Kategorie> {
    let mut treffer: Vec<Kategorie> = Vec::new();
    for fd in freedesktop {
        for k in abbilden(fd.as_ref()) {
            if !treffer.contains(k) {
                treffer.push(*k);
            }
        }
    }
    if treffer.is_empty() {
        return vec![Sonstiges];
    }
    // Anzeigereihenfolge wie `Kategorie::alle()`.
    let mut sortiert: Vec<Kategorie> = Kategorie::alle().iter().copied().filter(|k| treffer.contains(k)).collect();
    if sortiert.is_empty() {
        sortiert.push(Sonstiges);
    }
    sortiert
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hauptkategorien() {
        assert_eq!(abbilden("Utility"), &[Zubehoer]);
        assert_eq!(abbilden("Office"), &[Buero]);
        assert_eq!(abbilden("Graphics"), &[Grafik]);
        assert_eq!(abbilden("Network"), &[Internet]);
        assert_eq!(abbilden("AudioVideo"), &[Multimedia]);
        assert_eq!(abbilden("Game"), &[Spiele]);
        assert_eq!(abbilden("Development"), &[Entwicklung]);
        assert_eq!(abbilden("Education"), &[Bildung]);
        assert_eq!(abbilden("Science"), &[Bildung]);
        assert_eq!(abbilden("System"), &[System]);
        assert_eq!(abbilden("Settings"), &[System]);
    }

    #[test]
    fn mehrfach_und_reihenfolge() {
        assert_eq!(zuordnen(&["Development", "Database", "Office"]), vec![Buero, Entwicklung]);
        assert_eq!(zuordnen(&["Game", "Utility"]), vec![Zubehoer, Spiele]);
    }

    #[test]
    fn rest_ist_sonstiges() {
        assert_eq!(zuordnen(&["GTK", "Qt"]), vec![Sonstiges]);
        assert_eq!(zuordnen::<&str>(&[]), vec![Sonstiges]);
        assert_eq!(zuordnen(&["X-Unbekannt"]), vec![Sonstiges]);
        assert_eq!(zuordnen(&["GTK", "Utility"]), vec![Zubehoer]);
    }

    #[test]
    fn gross_klein_egal() {
        assert_eq!(abbilden("utility"), &[Zubehoer]);
        assert_eq!(abbilden(" TextEditor "), &[Zubehoer]);
    }

    #[test]
    fn tabelle_ohne_dubletten_und_ohne_leere_ziele() {
        for (i, (name, ziel)) in ABBILDUNG.iter().enumerate() {
            assert!(!ziel.is_empty(), "{} ohne Ziel", name);
            assert!(
                !ABBILDUNG[..i].iter().any(|(n, _)| n.eq_ignore_ascii_case(name)),
                "{} doppelt in der Tabelle",
                name
            );
        }
    }
}
