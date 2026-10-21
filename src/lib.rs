//! # qs_katalog – AppStream-Katalog für Querencia Software
//!
//! Liest die AppStream-Daten des Systems (DEP-11-YAML, Katalog-XML, MetaInfo-XML), filtert auf
//! Anwendungen mit Debian-Paket, bildet Freedesktop-Kategorien auf die zehn Querencia-Kategorien
//! ab, bietet eine gewichtete Suche mit Umlaut-Toleranz und löst Symbole auf.
//!
//! Einstieg: [`AppStreamKatalog::neu`] und das Trait [`Katalog`] aus `qs_typen` (SW-1), beide
//! nach `docs/SCHNITTSTELLEN.md` (Repo querencia-software). Bedeutung der [`Statistik`]-Felder
//! siehe [`AppStreamKatalog`].
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
pub use qs_typen::{Anwendung, Katalog, Kategorie, Statistik, Symbol};

