//! Vorläufiger Rauchtest: alle DEP-11-Dateien lesen und zählen.
fn main() {
    let start = std::time::Instant::now();
    let mut gesamt = 0;
    for e in std::fs::read_dir("/var/lib/swcatalog/yaml").unwrap() {
        let p = e.unwrap().path();
        let t = std::time::Instant::now();
        match qs_katalog::dep11::lese_datei(&p) {
            Ok((kopf, k)) => {
                gesamt += k.len();
                println!("{:60} {:5} Komponenten  {:?}  {:?}", kopf.origin, k.len(), t.elapsed(), p.file_name().unwrap());
            }
            Err(e) => println!("FEHLER {}", e),
        }
    }
    println!("gesamt {} in {:?}", gesamt, start.elapsed());
}
