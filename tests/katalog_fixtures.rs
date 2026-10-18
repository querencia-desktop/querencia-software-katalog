//! Tests mit kleinen Fixture-Dateien: Einlesen, Filter, Locale-Fallback, Kategorien, Suche, Symbole.

use std::path::PathBuf;

use qs_katalog::{AppStreamKatalog, Katalog, Kategorie, Konfiguration, Symbol};

fn fixture(unter: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(unter)
}

fn geladen() -> (AppStreamKatalog, qs_katalog::Statistik) {
    let mut k = AppStreamKatalog::mit_konfiguration(Konfiguration {
        katalog_verzeichnisse: vec![fixture("katalog")],
        metainfo_verzeichnisse: vec![fixture("metainfo")],
        symbol_cache_verzeichnisse: vec![fixture("icons")],
        themen_verzeichnisse: vec![fixture("themen")],
        mit_konsolenanwendungen: true,
    });
    let s = k.laden().expect("Fixtures laden");
    (k, s)
}

#[test]
fn zaehlung_und_filter() {
    let (k, s) = geladen();
    // YAML: 9 Komponenten, XML: 3 lesbare (die kaputte Datei wird gemeldet, nicht fatal).
    assert_eq!(s.komponenten, 12);
    // Sichtbar: Notizen, Büroklammer, Spiel, Fremdes, Zeilenwerkzeug, Wörterbuch, English Only.
    assert_eq!(s.mit_paket, 7, "{:?}", k.alle().iter().map(|a| &a.id).collect::<Vec<_>>());
    let b = k.ladebericht();
    assert_eq!(b.ausgeblendet_bundle, 1);
    assert_eq!(b.ausgeblendet_ohne_paket, 2); // YAML ohne Package + XML ohne pkgname
    assert_eq!(b.ausgeblendet_typ, 2); // font + addon
    assert_eq!(b.konsolenanwendungen, 1);
    assert_eq!(b.fehler.len(), 1, "kaputte MetaInfo-Datei muss gemeldet werden");
    assert!(b.fehler[0].0.ends_with("kaputt.metainfo.xml"));
    assert_eq!(s.quellen.len(), 4);
    assert!(k.nach_id("org.example.SnapApp.desktop").is_none());
    assert!(k.nach_id("org.example.OhnePaket.desktop").is_none());
    assert!(k.nach_id("org.example.OhnePaketXml").is_none());
    assert!(k.nach_id("org.example.Schrift").is_none());
}

#[test]
fn ohne_konsolenanwendungen() {
    let mut k = AppStreamKatalog::mit_konfiguration(Konfiguration {
        katalog_verzeichnisse: vec![fixture("katalog")],
        metainfo_verzeichnisse: vec![],
        symbol_cache_verzeichnisse: vec![],
        themen_verzeichnisse: vec![],
        mit_konsolenanwendungen: false,
    });
    let s = k.laden().unwrap();
    assert_eq!(s.mit_paket, 4);
    assert!(k.nach_paket("zeilenwerkzeug").is_none());
}

#[test]
fn locale_fallback() {
    let (k, _) = geladen();
    // de vorhanden
    let n = k.nach_paket("notizen").unwrap();
    assert_eq!(n.name, "Notizen");
    assert_eq!(n.zusammenfassung, "Schnelle Notizen anfertigen");
    // nur de_DE (und C) vorhanden → de_DE
    let b = k.nach_paket("bueroklammer").unwrap();
    assert_eq!(b.name, "Büroklammer");
    // nur C → C
    let s = k.nach_paket("spiel").unwrap();
    assert_eq!(s.name, "Space Blocks");
    assert_eq!(s.beschreibung, "Only English here.");
    // MetaInfo mit nur englischen Texten
    let e = k.nach_paket("englisch-app").unwrap();
    assert_eq!(e.name, "English Only");
    assert_eq!(e.zusammenfassung, "Only English texts");
    assert_eq!(e.beschreibung, "No translations exist.");
    // MetaInfo mit Deutsch (absatzweise lokalisiert)
    let d = k.nach_paket("deutsch-app").unwrap();
    assert_eq!(d.name, "Wörterbuch");
    assert_eq!(d.zusammenfassung, "Wörter nachschlagen");
    assert_eq!(d.beschreibung, "Schlagen Sie Wörter schnell nach.");
}

#[test]
fn statistik_deutsch() {
    let (_, s) = geladen();
    // Deutsche Zusammenfassung: Notizen (de), Büroklammer (de_DE), Wörterbuch (de).
    assert_eq!(s.deutsch, 3);
}

#[test]
fn beschreibung_als_text() {
    let (k, _) = geladen();
    let n = k.nach_paket("notizen").unwrap();
    assert_eq!(
        n.beschreibung,
        "Schreiben Sie schnelle Notizen auf. Sie bleiben & erhalten.\n\n• Einfach\n• Schnell"
    );
}

#[test]
fn felder_der_anwendung() {
    let (k, _) = geladen();
    let n = k.nach_id("org.example.Notizen.desktop").unwrap();
    assert_eq!(n.paketname.as_deref(), Some("notizen"));
    assert_eq!(n.lizenz.as_deref(), Some("GPL-3.0+"));
    assert_eq!(n.homepage.as_deref(), Some("https://notizen.example"));
    assert_eq!(n.bildschirmfotos, ["https://medien.example/media/n/o/shot.png"]);
    // nach_id auch ohne ".desktop"
    assert_eq!(k.nach_id("org.example.Notizen").unwrap().id, n.id);
    let z = k.zusatz(n).unwrap();
    assert_eq!(z.typ, "desktop-application");
    assert_eq!(z.origin, "test-origin");
    assert_eq!(z.schluesselwoerter, ["Merkzettel", "Gedächtnis", "memo"]);
    assert_eq!(z.symbol_fern_url.as_deref(), Some("https://medien.example/media/n/o/notizen.png"));
    assert!(k.nach_paket("gibtesnicht").is_none());
}

#[test]
fn kategorien_zuordnung() {
    let (k, _) = geladen();
    assert_eq!(k.nach_paket("notizen").unwrap().kategorien, [Kategorie::Zubehoer]);
    assert_eq!(
        k.nach_paket("bueroklammer").unwrap().kategorien,
        [Kategorie::Buero, Kategorie::Entwicklung]
    );
    assert_eq!(k.nach_paket("spiel").unwrap().kategorien, [Kategorie::Spiele]);
    assert_eq!(k.nach_paket("fremd").unwrap().kategorien, [Kategorie::Sonstiges]);
    assert_eq!(k.nach_paket("zeilenwerkzeug").unwrap().kategorien, [Kategorie::System]);
    assert_eq!(k.nach_paket("deutsch-app").unwrap().kategorien, [Kategorie::Zubehoer, Kategorie::Bildung]);
    assert_eq!(k.nach_paket("englisch-app").unwrap().kategorien, [Kategorie::Internet]);
}

#[test]
fn kategorie_liste_sortiert_nach_name() {
    let (k, _) = geladen();
    let namen: Vec<&str> = k.kategorie(Kategorie::Zubehoer).iter().map(|a| a.name.as_str()).collect();
    assert_eq!(namen, ["Notizen", "Wörterbuch"]);
    let namen: Vec<&str> = k.alle().iter().map(|a| a.name.as_str()).collect();
    let mut sortiert = namen.clone();
    sortiert.sort_by_key(|n| qs_katalog::suche::normalisieren(n));
    assert_eq!(namen, sortiert);
    assert_eq!(k.kategorie(Kategorie::Sonstiges).len(), 1);
}

#[test]
fn suche_mit_umlauten() {
    let (k, _) = geladen();
    for text in ["Büroklammer", "bueroklammer", "BÜRO", "buero", "buro", "Büro"] {
        let t = k.suchen(text);
        assert_eq!(t.first().map(|a| a.id.as_str()), Some("org.example.Buero.desktop"), "Suche {:?}: {:?}", text, t.len());
    }
    // ä/ae, ö/oe: „Wörterbuch“ über Umschrift und ohne Umlaut
    for text in ["wörter", "woerter", "WOERTERBUCH", "worter"] {
        let t = k.suchen(text);
        assert_eq!(t.first().map(|a| a.paketname.as_deref()), Some(Some("deutsch-app")), "Suche {:?}", text);
    }
    // Schlüsselwort mit Umlaut
    let t = k.suchen("gedaechtnis");
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].paketname.as_deref(), Some("notizen"));
}

#[test]
fn suche_gewichtung_und_stabilitaet() {
    let (k, _) = geladen();
    // „notiz“: Name-Präfix (Notizen) vor Beschreibung/Zusammenfassung anderer.
    let t = k.suchen("notiz");
    assert_eq!(t[0].paketname.as_deref(), Some("notizen"));
    // Schlüsselwort „memo“ (C-Fassung) findet Notizen.
    assert_eq!(k.suchen("memo")[0].paketname.as_deref(), Some("notizen"));
    // Paketname: „zeilenwerkzeug“ ist auch der Name; „fremd“ trifft Paketname/Name.
    assert_eq!(k.suchen("zeilen")[0].paketname.as_deref(), Some("zeilenwerkzeug"));
    // Mehrere Wörter müssen alle treffen.
    assert_eq!(k.suchen("space blocks").len(), 1);
    assert!(k.suchen("space gibtesnicht").is_empty());
    // Leere Eingabe: keine Treffer.
    assert!(k.suchen("   ").is_empty());
    // Wiederholbar und stabil.
    let a: Vec<_> = k.suchen("e").iter().map(|a| a.id.clone()).collect();
    let b: Vec<_> = k.suchen("e").iter().map(|a| a.id.clone()).collect();
    assert_eq!(a, b);
}

#[test]
fn suche_hoechstens_200_treffer() {
    // 250 künstliche Komponenten mit gleichem Suchwort.
    let verzeichnis = std::env::temp_dir().join(format!("qs_katalog_viele_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&verzeichnis);
    std::fs::create_dir_all(&verzeichnis).unwrap();
    let mut yaml = String::from("---\nFile: DEP-11\nVersion: '1.0'\nOrigin: viele\n");
    for i in 0..250 {
        yaml.push_str(&format!(
            "---\nType: desktop-application\nID: viele.App{i:03}.desktop\nPackage: app{i:03}\nName:\n  C: Gemeinsam {i:03}\nSummary:\n  C: x\nCategories:\n- Utility\n"
        ));
    }
    std::fs::write(verzeichnis.join("viele.yml"), yaml).unwrap();
    let mut k = AppStreamKatalog::mit_konfiguration(Konfiguration {
        katalog_verzeichnisse: vec![verzeichnis.clone()],
        metainfo_verzeichnisse: vec![],
        symbol_cache_verzeichnisse: vec![],
        themen_verzeichnisse: vec![],
        mit_konsolenanwendungen: true,
    });
    assert_eq!(k.laden().unwrap().mit_paket, 250);
    let t = k.suchen("gemeinsam");
    assert_eq!(t.len(), 200);
    // Stabil: gleiche Gewichte behalten Namensreihenfolge.
    assert_eq!(t[0].name, "Gemeinsam 000");
    assert_eq!(t[199].name, "Gemeinsam 199");
    let _ = std::fs::remove_dir_all(&verzeichnis);
}

#[test]
fn symbole() {
    let (k, s) = geladen();
    // Themensymbole: Name bleibt erhalten, auch wenn die Datei fehlt.
    let n = k.nach_paket("notizen").unwrap();
    assert_eq!(n.symbol, Symbol::Thema("notizen".into()));
    assert_eq!(k.symbol_pfad(n, 48), None);
    // Cache-Symbol: Datei in der gewünschten Größe.
    let b = k.nach_paket("bueroklammer").unwrap();
    assert!(matches!(&b.symbol, Symbol::Datei(p) if p.ends_with("test-origin/64x64/buero_buero.png")), "{:?}", b.symbol);
    assert!(k.symbol_pfad(b, 48).unwrap().ends_with("test-origin/48x48/buero_buero.png"));
    assert!(k.symbol_pfad(b, 64).unwrap().ends_with("test-origin/64x64/buero_buero.png"));
    // Kein Symbol.
    let f = k.nach_paket("fremd").unwrap();
    assert_eq!(f.symbol, Symbol::Keines);
    assert_eq!(k.symbol_pfad(f, 48), None);
    // Statistik zählt nur tatsächlich gefundene Dateien.
    assert_eq!(s.mit_symbol, 1);
}

#[test]
fn leere_quellen_sind_ein_fehler() {
    let mut k = AppStreamKatalog::mit_konfiguration(Konfiguration {
        katalog_verzeichnisse: vec![PathBuf::from("/gibt/es/nicht")],
        metainfo_verzeichnisse: vec![],
        symbol_cache_verzeichnisse: vec![],
        themen_verzeichnisse: vec![],
        mit_konsolenanwendungen: true,
    });
    assert!(k.laden().is_err());
}

#[test]
fn spaetere_quelle_ueberschreibt() {
    let (k, _) = geladen();
    // Dieselbe ID in zwei Dateien: die später gelesene gewinnt.
    let verzeichnis = std::env::temp_dir().join(format!("qs_katalog_dubletten_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&verzeichnis);
    std::fs::create_dir_all(&verzeichnis).unwrap();
    let kopf = "---\nFile: DEP-11\nVersion: '1.0'\nOrigin: o\n---\nType: desktop-application\nID: a.desktop\nPackage: a\nSummary:\n  C: s\nName:\n  C: ";
    std::fs::write(verzeichnis.join("archive_dists_resolute_main.yml"), format!("{kopf}Alt\n")).unwrap();
    std::fs::write(verzeichnis.join("archive_dists_resolute-updates_main.yml"), format!("{kopf}Neu\n")).unwrap();
    let mut k2 = AppStreamKatalog::mit_konfiguration(Konfiguration {
        katalog_verzeichnisse: vec![verzeichnis.clone()],
        metainfo_verzeichnisse: vec![],
        symbol_cache_verzeichnisse: vec![],
        themen_verzeichnisse: vec![],
        mit_konsolenanwendungen: true,
    });
    let s = k2.laden().unwrap();
    assert_eq!(s.komponenten, 1);
    assert_eq!(k2.nach_paket("a").unwrap().name, "Neu");
    let _ = (k, std::fs::remove_dir_all(&verzeichnis));
}
