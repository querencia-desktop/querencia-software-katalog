//! Auflösung von Symbolen: Katalog-Cache (`cached`), Themensymbole (`stock`) und lokale Dateien.
//!
//! Die Oberfläche soll `Symbol::Thema(name)` bevorzugt über `gtk::IconTheme` anzeigen; die hier
//! gesuchten Pfade sind ein Rückfall und dienen der Statistik (ist überhaupt ein Bild da?).
//! `remote`-Symbole werden offline nicht geladen; ihre URL steht in `Zusatz::symbol_fern_url`.

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

/// Sucht ein Themensymbol als Datei: `<thema>/<s>x<s>/apps/<name>.{png,svg,xpm}`, dann
/// `scalable`, dann andere Größen, zuletzt `/usr/share/pixmaps`.
pub fn thema_pfad(themen_verzeichnisse: &[PathBuf], name: &str, groesse: u32) -> Option<PathBuf> {
    if name.is_empty() || name.contains('/') {
        // Absoluter Pfad als „stock“-Name kommt vor (fehlerhafte Desktop-Dateien).
        let p = Path::new(name);
        return if p.is_absolute() && p.is_file() { Some(p.to_path_buf()) } else { None };
    }
    let mut groessen = vec![groesse];
    for g in [64u32, 48, 128, 256, 96, 72, 32, 24, 22, 16] {
        if !groessen.contains(&g) {
            groessen.push(g);
        }
    }
    let name_ohne_endung = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".svg"))
        .or_else(|| name.strip_suffix(".xpm"))
        .unwrap_or(name);
    for thema in themen_verzeichnisse {
        for g in &groessen {
            if *g == groesse || g > &groesse {
                for endung in ["png", "svg"] {
                    let p = thema.join(format!("{}x{}/apps/{}.{}", g, g, name_ohne_endung, endung));
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
        let p = thema.join(format!("scalable/apps/{}.svg", name_ohne_endung));
        if p.is_file() {
            return Some(p);
        }
        for g in &groessen {
            if g < &groesse {
                for endung in ["png", "svg"] {
                    let p = thema.join(format!("{}x{}/apps/{}.{}", g, g, name_ohne_endung, endung));
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }
    for endung in ["png", "svg", "xpm"] {
        let p = PathBuf::from(format!("/usr/share/pixmaps/{}.{}", name_ohne_endung, endung));
        if p.is_file() {
            return Some(p);
        }
    }
    None
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
        let _ = fs::remove_dir_all(&basis);
    }
}
