//! # qs_katalog – AppStream-Katalog für Querencia Software
//!
//! Liest die AppStream-Daten des Systems (DEP-11-YAML, Katalog-XML, MetaInfo-XML), filtert auf
//! Anwendungen mit Debian-Paket, bildet Freedesktop-Kategorien auf die zehn Querencia-Kategorien
//! ab, bietet eine gewichtete Suche mit Umlaut-Toleranz und löst Symbole auf.
//!
//! Einstieg: [`AppStreamKatalog::neu`] und das Trait [`Katalog`]. Die Signaturen stammen aus
//! `~/projects/querencia-software/SCHNITTSTELLEN.md`; die gemeinsamen Typen kommen aus `qs_typen`.
//!
//! ```no_run
//! use qs_katalog::{AppStreamKatalog, Katalog};
//! let mut katalog = AppStreamKatalog::neu();
//! let statistik = katalog.laden().expect("AppStream-Daten lesen");
//! println!("{} Anwendungen", statistik.mit_paket);
//! for a in katalog.suchen("Texteditor").iter().take(5) {
//!     println!("{} – {}", a.name, a.zusammenfassung);
//! }
//! ```

pub mod beschreibung;
pub mod dep11;
pub mod katalog;
pub mod kategorien;
pub mod komponente;
pub mod suche;
pub mod symbole;
pub mod xml;
pub mod yaml;

pub use katalog::{AppStreamKatalog, Konfiguration, Ladebericht, Zusatz};
pub use qs_typen::{Anwendung, Kategorie, Symbol};

/// Kennzahlen nach dem Laden (Signatur aus SCHNITTSTELLEN.md).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Statistik {
    /// Eindeutige Komponenten aller Typen in allen Quellen (gleiche ID zählt einmal).
    pub komponenten: usize,
    /// Sichtbare Anwendungen: `desktop-application`/`console-application` mit Paket, kein Bundle.
    pub mit_paket: usize,
    /// Sichtbare Anwendungen, für die lokal eine Symboldatei gefunden wurde
    /// (Katalog-Cache oder Themenverzeichnis) – nicht nur ein Symbolname.
    pub mit_symbol: usize,
    /// Sichtbare Anwendungen mit deutscher Zusammenfassung (`de` oder `de_DE`).
    pub deutsch: usize,
    /// Gelesene Quelldateien als `"<Pfad> (<n> Komponenten)"`, in Leserreihenfolge.
    pub quellen: Vec<String>,
}

/// Der Katalog, wie ihn die Oberfläche (SW-3) benutzt. Signatur aus SCHNITTSTELLEN.md.
pub trait Katalog {
    /// Liest alle Quellen und baut die Indizes; darf 1–2 s dauern und wird im Hintergrund-Thread gerufen.
    fn laden(&mut self) -> Result<Statistik, String>;
    /// Anwendungen einer Kategorie, nach Name sortiert.
    fn kategorie(&self, k: Kategorie) -> Vec<&Anwendung>;
    /// Suche über Name, Zusammenfassung, Schlüsselwörter, Paketname, Beschreibung; Treffer nach Gewicht, höchstens 200.
    fn suchen(&self, text: &str) -> Vec<&Anwendung>;
    /// Anwendung zu einem Paketnamen (bei mehreren Desktop-Dateien je Paket: die erste nach Name).
    fn nach_paket(&self, paketname: &str) -> Option<&Anwendung>;
    /// Anwendung zu einer Component-ID (`org.gnome.gedit.desktop`; auch ohne `.desktop` auffindbar).
    fn nach_id(&self, id: &str) -> Option<&Anwendung>;
    /// Pfad zu einer Symboldatei in der gewünschten Größe (48 oder 64), falls lokal vorhanden.
    fn symbol_pfad(&self, a: &Anwendung, groesse: u32) -> Option<std::path::PathBuf>;
    /// Alle sichtbaren Anwendungen, nach Name sortiert.
    fn alle(&self) -> &[Anwendung];
}
