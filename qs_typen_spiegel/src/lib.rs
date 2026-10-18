//! Lokaler Spiegel der gemeinsamen Typen aus `~/projects/querencia-software/SCHNITTSTELLEN.md`.
//!
//! Dieses Crate heißt absichtlich `qs_typen`, damit `qs_katalog` ohne Codeänderung auf das
//! echte Crate von SW-1 umgestellt werden kann (nur der `path` in `Cargo.toml` wechselt).
//! Inhalt wörtlich nach dem Dokument, Stand 18.10.2026, 20:20.

use std::path::PathBuf;

/// PackageKit-Format `"name;version;arch;data"`.
pub type PaketId = String;

#[derive(Clone, Debug, PartialEq)]
pub struct Paket {
    pub id: PaketId,
    /// Paketname, z. B. `"querencia-bilder"`.
    pub name: String,
    pub version: String,
    pub arch: String,
    /// Eine Zeile.
    pub zusammenfassung: String,
    pub installiert: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Paketdetails {
    pub paket: Paket,
    /// Mehrzeilig.
    pub beschreibung: String,
    pub lizenz: String,
    /// Download- oder Installationsgröße, je nach Backend.
    pub groesse_bytes: u64,
    pub url: Option<String>,
    /// PackageKit-Group als Text.
    pub gruppe: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Dringlichkeit {
    Normal,
    Sicherheit,
    Fehlerbehebung,
    Erweiterung,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aktualisierung {
    pub paket: Paket,
    pub dringlichkeit: Dringlichkeit,
    pub neustart_noetig: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Vorgang {
    Installieren,
    Entfernen,
    Aktualisieren,
    QuellenAuffrischen,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fortschritt {
    Gestartet { vorgang: Vorgang, pakete: Vec<PaketId> },
    /// 0..=100, 101 = unbekannt (PackageKit-Konvention) → als None behandeln.
    Prozent(u8),
    /// PackageKit-Status als deutscher Text („Lade herunter …“).
    Status(String),
    PaketFortschritt { id: PaketId, prozent: u8 },
    /// cannot-get-lock → Oberfläche bietet Warten/Abbrechen.
    WartetAufSperre,
    BrauchtNeustart,
    Fertig,
    Abgebrochen,
    Fehler(Fehler),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Fehler {
    /// polkit abgelehnt oder kein Agent.
    NichtBerechtigt,
    /// polkit-Agent fehlt (G2-I 1.4): eigene Meldung mit Hinweis.
    KeinAgent,
    SperreBelegt(String),
    NetzFehlt,
    PaketNichtGefunden(String),
    AbhaengigkeitenKonflikt(String),
    /// PackageKit beendet/neu gestartet während der Transaktion.
    DaemonWeg,
    Abgebrochen,
    /// PackageKit-ErrorCode + Details.
    Sonstiges { code: String, text: String },
}

/// Eine Anwendung aus AppStream.
#[derive(Clone, Debug, PartialEq)]
pub struct Anwendung {
    /// Component-ID, z. B. `"org.querencia.Bilder"`.
    pub id: String,
    /// Lokalisiert (de, sonst C).
    pub name: String,
    pub zusammenfassung: String,
    /// Bereits als einfacher Text (Entscheidung SW-2: reiner Text mit Absätzen und „• “).
    pub beschreibung: String,
    /// `<pkgname>`; None = ohne Paket (z. B. Snap-Komponenten, werden ausgeblendet).
    pub paketname: Option<String>,
    pub kategorien: Vec<Kategorie>,
    pub symbol: Symbol,
    /// URLs, v1 zeigt sie nicht an, aber sammelt sie.
    pub bildschirmfotos: Vec<String>,
    pub lizenz: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Symbol {
    Thema(String),
    Datei(PathBuf),
    Keines,
}

/// Feste Kategorienliste nach GNOME 2 (Reihenfolge = Anzeige in der Seitenleiste).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kategorie {
    Zubehoer,
    Buero,
    Grafik,
    Internet,
    Multimedia,
    Spiele,
    Entwicklung,
    Bildung,
    System,
    Sonstiges,
}

impl Kategorie {
    /// Anzeigename für die Seitenleiste.
    pub fn name(self) -> &'static str {
        match self {
            Kategorie::Zubehoer => "Zubehör",
            Kategorie::Buero => "Büro",
            Kategorie::Grafik => "Grafik",
            Kategorie::Internet => "Internet",
            Kategorie::Multimedia => "Multimedia",
            Kategorie::Spiele => "Spiele",
            Kategorie::Entwicklung => "Entwicklung",
            Kategorie::Bildung => "Bildung",
            Kategorie::System => "System",
            Kategorie::Sonstiges => "Sonstiges",
        }
    }

    /// Alle Kategorien in Anzeigereihenfolge.
    pub fn alle() -> &'static [Kategorie] {
        const ALLE: [Kategorie; 10] = [
            Kategorie::Zubehoer,
            Kategorie::Buero,
            Kategorie::Grafik,
            Kategorie::Internet,
            Kategorie::Multimedia,
            Kategorie::Spiele,
            Kategorie::Entwicklung,
            Kategorie::Bildung,
            Kategorie::System,
            Kategorie::Sonstiges,
        ];
        &ALLE
    }
}
