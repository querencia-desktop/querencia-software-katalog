//! Werkzeug zum Katalog: Statistik ausgeben, suchen, Kategorie auflisten.
//!
//! ```text
//! cargo run --release --example katalog-suche                      # Statistik
//! cargo run --release --example katalog-suche -- suche gimp        # Suche
//! cargo run --release --example katalog-suche -- kategorie Büro    # Kategorie auflisten
//! cargo run --release --example katalog-suche -- id org.gnome.gedit.desktop
//! cargo run --release --example katalog-suche -- paket gedit
//! cargo run --release --example katalog-suche -- quellen           # gelesene Dateien
//! cargo run --release --example katalog-suche -- dump              # alle Anwendungen vollständig
//! cargo run --release --example katalog-suche -- doppel            # IDs mit und ohne „.desktop“
//! ```
//! Mit `QS_KATALOG_VERZEICHNIS=<pfad>` wird statt der Systempfade nur dieses Verzeichnis
//! (Katalogdateien und MetaInfo) gelesen.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use qs_katalog::{AppStreamKatalog, Katalog, Kategorie, Konfiguration, Symbol};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut katalog = match std::env::var_os("QS_KATALOG_VERZEICHNIS") {
        Some(v) => {
            let mut k = Konfiguration::system();
            k.katalog_verzeichnisse = vec![v.clone().into()];
            k.metainfo_verzeichnisse = vec![v.into()];
            AppStreamKatalog::mit_konfiguration(k)
        }
        None => AppStreamKatalog::neu(),
    };
    let start = Instant::now();
    let statistik = match katalog.laden() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Laden fehlgeschlagen: {}", e);
            std::process::exit(1);
        }
    };
    let ladezeit = start.elapsed();

    match args.first().map(String::as_str) {
        None | Some("statistik") => {
            let b = katalog.ladebericht();
            println!("Ladezeit: {:?}", ladezeit);
            println!(
                "Quellen: {} Dateien, {} Einträge gelesen",
                statistik.quellen.len(),
                b.eintraege_gelesen
            );
            println!("Komponenten (eindeutig): {}", statistik.komponenten);
            for (typ, n) in &b.je_typ {
                println!("  {:24} {:5}", typ, n);
            }
            println!("Sichtbare Anwendungen (mit Paket): {}", statistik.mit_paket);
            println!("  davon Konsolenanwendungen: {}", b.konsolenanwendungen);
            println!("  mit deutscher Zusammenfassung: {}", statistik.deutsch);
            println!("  mit Themensymbol (stock): {}", b.mit_themensymbol);
            println!("  mit Symbol aus dem Katalog-Cache: {}", b.mit_cache_symbol);
            println!(
                "  mit lokal gefundener Symboldatei: {}",
                statistik.mit_symbol
            );
            println!(
                "Ausgeblendet: {} anderer Typ, {} ohne Paket, {} Bundle",
                b.ausgeblendet_typ, b.ausgeblendet_ohne_paket, b.ausgeblendet_bundle
            );
            println!("Je Kategorie:");
            for (k, n) in &b.je_kategorie {
                println!("  {:20} {:5}", k.name(), n);
            }
            if !b.fehler.is_empty() {
                println!("Fehler:");
                for (_, e) in &b.fehler {
                    println!("  {}", e);
                }
            }
        }
        Some("dump") => {
            // Alle Anwendungen vollständig, für Vergleiche zwischen Einlesewegen.
            for a in katalog.alle() {
                let z = katalog.zusatz(a).expect("zusatz");
                println!(
                    "{:?} | {} | {:?} | {:?} | {:?}",
                    a, z.typ, z.schluesselwoerter, z.symbol_fern_url, z.symbole_cache
                );
            }
        }
        Some("doppel") => {
            // Kennungen, die in den Quellen mit und ohne „.desktop“ vorkommen, und Anwendungen,
            // die im geladenen Katalog mehrfach unter derselben Kennung ohne „.desktop“ stehen.
            let mut schreibweisen: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            let k = katalog.konfiguration();
            for v in k
                .katalog_verzeichnisse
                .iter()
                .chain(&k.metainfo_verzeichnisse)
            {
                let Ok(eintraege) = std::fs::read_dir(v) else {
                    continue;
                };
                for e in eintraege.filter_map(Result::ok) {
                    let p = e.path();
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let gelesen = if name.contains(".yml") || name.contains(".yaml") {
                        qs_katalog::dep11::lese_datei(&p)
                    } else if name.ends_with(".xml") || name.ends_with(".xml.gz") {
                        qs_katalog::xml::lese_datei(&p)
                    } else {
                        continue;
                    };
                    for roh in gelesen.map(|(_, k)| k).unwrap_or_default() {
                        let kurz = roh.id.strip_suffix(".desktop").unwrap_or(&roh.id);
                        schreibweisen
                            .entry(kurz.to_string())
                            .or_default()
                            .insert(roh.id.clone());
                    }
                }
            }
            let quellen_doppelt: Vec<_> = schreibweisen
                .iter()
                .filter(|(_, s)| s.len() > 1)
                .map(|(k, _)| k)
                .collect();
            let mut je_kennung: BTreeMap<&str, usize> = BTreeMap::new();
            for a in katalog.alle() {
                let kurz = a.id.strip_suffix(".desktop").unwrap_or(&a.id);
                *je_kennung.entry(kurz).or_insert(0) += 1;
            }
            let katalog_doppelt: Vec<_> = je_kennung.iter().filter(|(_, n)| **n > 1).collect();
            println!("Komponenten (eindeutig): {}", statistik.komponenten);
            println!("Sichtbare Anwendungen: {}", statistik.mit_paket);
            println!(
                "Kennungen mit beiden Schreibweisen in den Quellen: {}",
                quellen_doppelt.len()
            );
            println!("Doppelte Anwendungen im Katalog: {}", katalog_doppelt.len());
            for (kennung, n) in &katalog_doppelt {
                println!("  {} ({}×)", kennung, n);
            }
        }
        Some("quellen") => {
            for q in &statistik.quellen {
                println!("{}", q);
            }
        }
        Some("suche") => {
            let text = args[1..].join(" ");
            let t = Instant::now();
            let treffer = katalog.suchen(&text);
            let dauer = t.elapsed();
            for a in &treffer {
                zeile(a);
            }
            println!(
                "{} Treffer für {:?} in {:?} (Laden {:?})",
                treffer.len(),
                text,
                dauer,
                ladezeit
            );
        }
        Some("kategorie") => {
            let name = args.get(1).map(String::as_str).unwrap_or("");
            let Some(k) = Kategorie::alle().iter().find(|k| {
                k.name().eq_ignore_ascii_case(name) || format!("{:?}", k).eq_ignore_ascii_case(name)
            }) else {
                eprintln!(
                    "Unbekannte Kategorie {:?}. Bekannt: {}",
                    name,
                    Kategorie::alle()
                        .iter()
                        .map(|k| k.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                std::process::exit(2);
            };
            let liste = katalog.kategorie(*k);
            for a in &liste {
                zeile(a);
            }
            println!("{} Anwendungen in {}", liste.len(), k.name());
        }
        Some("id") | Some("paket") => {
            let schluessel = args.get(1).map(String::as_str).unwrap_or("");
            let a = if args[0] == "id" {
                katalog.nach_id(schluessel)
            } else {
                katalog.nach_paket(schluessel)
            };
            match a {
                None => {
                    eprintln!("nicht gefunden: {}", schluessel);
                    std::process::exit(3);
                }
                Some(a) => {
                    println!("{:#?}", a);
                    if let Some(z) = katalog.zusatz(a) {
                        println!(
                            "Typ: {}  Origin: {}  Schlüsselwörter: {:?}",
                            z.typ, z.origin, z.schluesselwoerter
                        );
                        println!("Symbol fern: {:?}", z.symbol_fern_url);
                    }
                    println!("Symbolpfad 48: {:?}", katalog.symbol_pfad(a, 48));
                    println!("Symbolpfad 64: {:?}", katalog.symbol_pfad(a, 64));
                    println!("\n{}", a.beschreibung);
                }
            }
        }
        Some(andere) => {
            eprintln!("Unbekannter Befehl {:?}. Befehle: statistik, quellen, dump, doppel, suche TEXT, kategorie NAME, id ID, paket NAME", andere);
            std::process::exit(2);
        }
    }
}

fn zeile(a: &qs_katalog::Anwendung) {
    let symbol = match &a.symbol {
        Symbol::Thema(n) => format!("thema:{}", n),
        Symbol::Datei(p) => format!("datei:{}", p.display()),
        Symbol::Keines => "-".to_string(),
    };
    println!(
        "{:32} {:20} {:40} {}",
        kuerzen(&a.name, 32),
        kuerzen(a.paketname.as_deref().unwrap_or("-"), 20),
        kuerzen(&a.zusammenfassung, 40),
        symbol
    );
}

fn kuerzen(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut k: String = s.chars().take(n - 1).collect();
        k.push('…');
        k
    }
}
