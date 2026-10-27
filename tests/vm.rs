//! Integrationstest gegen die echten AppStream-Daten einer Test-VM.
//!
//! Läuft nur auf Anforderung: `cargo test --release --test vm -- --ignored --nocapture`
//! oder `skripte/vm-pruefung.sh`. Erwartung vom 18.10.2026: 2634 Komponenten
//! (`appstreamcli status`), 2215 Desktop- + 17 Konsolenanwendungen mit Paket. Stand 27.10.2026
//! nach Zusammenlegen der Schreibweisen mit und ohne `.desktop`: 2647 Komponenten.

use std::time::Instant;

use qs_katalog::{AppStreamKatalog, Katalog, Kategorie, Symbol};

#[test]
#[ignore = "braucht die AppStream-Daten der VM"]
fn echte_vm_daten() {
    let mut k = AppStreamKatalog::neu();
    let start = Instant::now();
    let s = k.laden().expect("laden");
    let dauer = start.elapsed();
    let b = k.ladebericht();
    println!("Ladezeit {:?}", dauer);
    println!("{:#?}", s);
    println!("je Typ {:?}", b.je_typ);
    println!("je Kategorie {:?}", b.je_kategorie);

    assert!(b.fehler.is_empty(), "Lesefehler: {:?}", b.fehler);
    assert!(
        (2600..=2700).contains(&s.komponenten),
        "Komponenten {}",
        s.komponenten
    );
    assert!(
        (2150..=2300).contains(&s.mit_paket),
        "mit Paket {}",
        s.mit_paket
    );
    assert!(
        s.deutsch * 100 / s.mit_paket >= 25,
        "Deutschanteil {} von {}",
        s.deutsch,
        s.mit_paket
    );
    assert!(s.quellen.len() >= 16);

    // Bekannte Einträge.
    let gedit = k.nach_paket("gedit").expect("gedit");
    assert_eq!(gedit.id, "org.gnome.gedit");
    assert_eq!(gedit.zusammenfassung, "Texteditor");
    assert!(gedit.kategorien.contains(&Kategorie::Zubehoer));
    assert_eq!(gedit.symbol, Symbol::Thema("org.gnome.gedit".into()));
    assert!(k.nach_id("org.gnome.gedit").is_some());
    assert!(k.nach_id("org.gnome.gedit.desktop").is_some());
    // Keine Kennung doppelt, keine mit Altschreibweise (QD-182: Remmina stand zweimal da).
    let mut ids: Vec<&str> = k.alle().iter().map(|a| a.id.as_str()).collect();
    assert!(ids.iter().all(|i| !i.ends_with(".desktop")));
    ids.sort_unstable();
    let vorher = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), vorher);
    let remmina: Vec<_> = k
        .suchen("remmina")
        .into_iter()
        .filter(|a| a.paketname.as_deref() == Some("remmina"))
        .collect();
    assert_eq!(remmina.len(), 1);
    assert_eq!(remmina[0].id, "org.remmina.Remmina");
    let gimp = k.suchen("gimp");
    assert_eq!(
        gimp.first().and_then(|a| a.paketname.as_deref()),
        Some("gimp")
    );

    // Jede Kategorie außer „Sonstiges“ ist gut gefüllt.
    for kat in Kategorie::alle() {
        let n = k.kategorie(*kat).len();
        if *kat != Kategorie::Sonstiges {
            assert!(n >= 100, "{} nur {}", kat.name(), n);
        }
    }
    // Suche mit Umlaut.
    assert!(!k.suchen("büro").is_empty());
    assert_eq!(k.suchen("büro").len(), k.suchen("buero").len());
    // Keine Markup-Reste in Beschreibungen.
    let reste = k
        .alle()
        .iter()
        .filter(|a| a.beschreibung.contains("<p>") || a.beschreibung.contains("</li>"))
        .count();
    assert_eq!(reste, 0);
    // Ladezeit: Ziel < 2 s im Release-Build; im Debug-Build großzügiger.
    let grenze = if cfg!(debug_assertions) { 15.0 } else { 2.0 };
    assert!(dauer.as_secs_f64() < grenze, "Ladezeit {:?}", dauer);
}
