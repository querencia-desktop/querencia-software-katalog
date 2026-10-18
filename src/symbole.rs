//! Auflösung von Symbolen: Katalog-Cache (`cached`), Themensymbole (`stock`) und lokale Dateien.
//!
//! Die Oberfläche soll `Symbol::Thema(name)` bevorzugt über `gtk::IconTheme` anzeigen; die hier
//! gesuchten Pfade sind ein Rückfall und dienen der Statistik (ist überhaupt ein Bild da?).
//! `remote`-Symbole werden offline nicht geladen; ihre URL steht in `Zusatz::symbol_fern_url`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::komponente::SymbolDatei;

/// Standardverzeichnisse des AppStream-Symbolcaches (appstream 1.x, Reihenfolge = Vorrang).
pub fn standard_cache_verzeichnisse() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/var/lib/swcatalog/icons"),
        PathBuf::from("/var/cache/swcatalog/icons"),
        PathBuf::from("/usr/share/swcatalog/icons"),
        PathBuf::from("/var/lib/app-info/icons"),
        PathBuf::from("/usr/share/app-info/icons"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        v.push(Path::new(&home).join(".cache/swcatalog/icons"));
    }
    v
}

/// Verzeichnisse mit Themensymbolen (hicolor zuerst, dann Rückfälle) und `pixmaps`.
pub fn standard_themen_verzeichnisse() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/usr/share/icons/hicolor"),
        PathBuf::from("/usr/share/icons/Adwaita"),
        PathBuf::from("/usr/share/icons/gnome"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        v.push(Path::new(&home).join(".local/share/icons/hicolor"));
    }
    v
}

/// Index aller Themensymbol-Dateien (`<thema>/<g>x<g>/apps/*`, `<thema>/scalable/apps/*`,
/// `/usr/share/pixmaps/*`). Einmal beim Laden aufgebaut, ersetzt er pro Anwendung Dutzende
/// `stat`-Aufrufe (gemessen: 177 000 `statx` je Ladevorgang ohne Index).
#[derive(Clone, Debug, Default)]
pub struct ThemenIndex {
    /// Name ohne Endung → (Thema-Rang, Größe; 0 = scalable, u32::MAX = pixmaps, Pfad).
    dateien: HashMap<String, Vec<(usize, u32, PathBuf)>>,
}

impl ThemenIndex {
    /// Liest die Verzeichnisstruktur der Themen und `/usr/share/pixmaps` ein.
    pub fn aufbauen(themen_verzeichnisse: &[PathBuf]) -> Self {
        Self::aufbauen_mit_pixmaps(themen_verzeichnisse, Some(Path::new("/usr/share/pixmaps")))
    }

    /// Wie [`ThemenIndex::aufbauen`], mit wählbarem `pixmaps`-Verzeichnis (Tests: `None`).
    pub fn aufbauen_mit_pixmaps(themen_verzeichnisse: &[PathBuf], pixmaps: Option<&Path>) -> Self {
        let mut idx = ThemenIndex::default();
        for (rang, thema) in themen_verzeichnisse.iter().enumerate() {
            let Ok(groessen) = std::fs::read_dir(thema) else { continue };
            for g in groessen.filter_map(Result::ok) {
                let gname = g.file_name().to_string_lossy().into_owned();
                let groesse = if gname == "scalable" {
                    0
                } else {
                    match gname.split_once('x').and_then(|(a, _)| a.parse::<u32>().ok()) {
                        Some(n) if !gname.contains('@') => n,
                        _ => continue,
                    }
                };
                idx.verzeichnis_lesen(&g.path().join("apps"), rang, groesse);
            }
        }
        if let Some(p) = pixmaps {
            idx.verzeichnis_lesen(p, themen_verzeichnisse.len(), u32::MAX);
        }
        idx
    }

    fn verzeichnis_lesen(&mut self, verzeichnis: &Path, rang: usize, groesse: u32) {
        let Ok(dateien) = std::fs::read_dir(verzeichnis) else { return };
        for d in dateien.filter_map(Result::ok) {
            let name = d.file_name().to_string_lossy().into_owned();
            let Some((stamm, endung)) = name.rsplit_once('.') else { continue };
            if !matches!(endung, "png" | "svg" | "xpm") || (groesse != u32::MAX && endung == "xpm") {
                continue;
            }
            self.dateien.entry(stamm.to_string()).or_default().push((rang, groesse, d.path()));
        }
    }

    /// Anzahl indizierter Symbolnamen.
    pub fn len(&self) -> usize {
        self.dateien.len()
    }

    /// Ob der Index leer ist.
    pub fn is_empty(&self) -> bool {
        self.dateien.is_empty()
    }

    /// Beste Datei für ein Themensymbol: Themenreihenfolge zuerst; innerhalb eines Themas die
    /// gewünschte Größe, dann größere (aufsteigend), dann `scalable`, dann kleinere (absteigend);
    /// `pixmaps` zuletzt. Absolute Pfade als Name werden direkt geprüft.
    pub fn suchen(&self, name: &str, groesse: u32) -> Option<PathBuf> {
        if name.contains('/') {
            let p = Path::new(name);
            return (p.is_absolute() && p.is_file()).then(|| p.to_path_buf());
        }
        let stamm = name
            .strip_suffix(".png")
            .or_else(|| name.strip_suffix(".svg"))
            .or_else(|| name.strip_suffix(".xpm"))
            .unwrap_or(name);
        let kandidaten = self.dateien.get(stamm)?;
        kandidaten
            .iter()
            .min_by_key(|(rang, g, pfad)| {
                let ordnung = if *g == u32::MAX {
                    (3, 0)
                } else if *g == groesse {
                    (0, 0)
                } else if *g > groesse {
                    (1, *g)
                } else if *g == 0 {
                    (2, 0)
                } else {
                    (2, u32::MAX - *g)
                };
                // PNG vor SVG bei gleicher Größe (schneller zu laden).
                (*rang, ordnung, pfad.extension().is_some_and(|e| e != "png"))
            })
            .map(|(_, _, p)| p.clone())
    }
}

/// Sucht ein `cached`-Symbol unter `<cache>/<origin>/<BxH>[@skala]/<name>`; bevorzugt die
/// gewünschte Größe, dann die nächstgrößere, dann irgendeine.
pub fn cache_pfad(
    cache_verzeichnisse: &[PathBuf],
    origin: &str,
    symbole: &[SymbolDatei],
    groesse: u32,
) -> Option<PathBuf> {
    if symbole.is_empty() {
        return None;
    }
    let mut kandidaten: Vec<&SymbolDatei> = symbole.iter().filter(|s| s.skala <= 1).collect();
    if kandidaten.is_empty() {
        kandidaten = symbole.iter().collect();
    }
    // Reihenfolge: genaue Größe, dann aufsteigend größere, dann absteigend kleinere.
    kandidaten.sort_by_key(|s| {
        if s.breite == groesse {
            (0, 0)
        } else if s.breite > groesse {
            (1, s.breite)
        } else {
            (2, u32::MAX - s.breite)
        }
    });
    for s in kandidaten {
        let unter = if s.skala > 1 {
            format!("{}x{}@{}", s.breite, s.hoehe, s.skala)
        } else {
            format!("{}x{}", s.breite, s.hoehe)
        };
        for basis in cache_verzeichnisse {
            let mut p = basis.join(origin);
            p.push(&unter);
            p.push(&s.name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Sucht ein Themensymbol als Datei (baut dafür einen Index; für viele Abfragen
/// [`ThemenIndex`] direkt verwenden).
pub fn thema_pfad(themen_verzeichnisse: &[PathBuf], name: &str, groesse: u32) -> Option<PathBuf> {
    ThemenIndex::aufbauen(themen_verzeichnisse).suchen(name, groesse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn testverzeichnis(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("qs_katalog_symbole_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn cache_bevorzugt_gewuenschte_groesse() {
        let basis = testverzeichnis("cache");
        for g in ["48x48", "64x64", "128x128"] {
            let d = basis.join("test-origin").join(g);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("a.png"), b"x").unwrap();
        }
        let symbole = vec![
            SymbolDatei { name: "a.png".into(), breite: 48, hoehe: 48, skala: 1 },
            SymbolDatei { name: "a.png".into(), breite: 64, hoehe: 64, skala: 1 },
            SymbolDatei { name: "a.png".into(), breite: 128, hoehe: 128, skala: 1 },
        ];
        let v = vec![basis.clone()];
        assert!(cache_pfad(&v, "test-origin", &symbole, 64).unwrap().ends_with("64x64/a.png"));
        assert!(cache_pfad(&v, "test-origin", &symbole, 48).unwrap().ends_with("48x48/a.png"));
        // 96 fehlt → nächstgrößere 128
        assert!(cache_pfad(&v, "test-origin", &symbole, 96).unwrap().ends_with("128x128/a.png"));
        assert_eq!(cache_pfad(&v, "anderer-origin", &symbole, 64), None);
        let _ = fs::remove_dir_all(&basis);
    }

    #[test]
    fn thema_sucht_groesse_dann_scalable() {
        let basis = testverzeichnis("thema");
        fs::create_dir_all(basis.join("48x48/apps")).unwrap();
        fs::create_dir_all(basis.join("scalable/apps")).unwrap();
        fs::write(basis.join("48x48/apps/foo.png"), b"x").unwrap();
        fs::write(basis.join("scalable/apps/bar.svg"), b"x").unwrap();
        let v = vec![basis.clone()];
        assert!(thema_pfad(&v, "foo", 48).unwrap().ends_with("48x48/apps/foo.png"));
        assert!(thema_pfad(&v, "foo", 64).unwrap().ends_with("48x48/apps/foo.png"));
        assert!(thema_pfad(&v, "bar", 64).unwrap().ends_with("scalable/apps/bar.svg"));
        assert!(thema_pfad(&v, "foo.png", 48).is_some());
        assert_eq!(thema_pfad(&v, "nix-da-ganz-sicher-nicht", 48), None);
        let idx = ThemenIndex::aufbauen_mit_pixmaps(&v, None);
        assert_eq!(idx.len(), 2);
        assert!(idx.suchen("bar", 48).unwrap().ends_with("scalable/apps/bar.svg"));
        let _ = fs::remove_dir_all(&basis);
    }
}
