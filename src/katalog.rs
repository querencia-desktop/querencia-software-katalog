//! Der eigentliche Katalog: Quellen lesen, filtern, umwandeln, indizieren, suchen.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use qs_typen::{Anwendung, Kategorie, Symbol};

use crate::komponente::{Quellkopf, RohKomponente, SymbolDatei};
use crate::suche::{self, Suchfelder};
use qs_typen::{Katalog, Statistik};

use crate::{beschreibung, dep11, kategorien, symbole, xml};

/// Höchstzahl der Suchtreffer.
pub const MAX_TREFFER: usize = 200;

/// Woher gelesen wird. [`Konfiguration::system`] entspricht SCHNITTSTELLEN.md; Tests setzen
/// eigene Verzeichnisse.
#[derive(Clone, Debug, PartialEq)]
pub struct Konfiguration {
    /// Katalogverzeichnisse mit `*.yml[.gz]` oder `*.xml[.gz]`, in Leserreihenfolge
    /// (spätere Quellen überschreiben gleiche IDs).
    pub katalog_verzeichnisse: Vec<PathBuf>,
    /// Verzeichnisse mit MetaInfo-Dateien (`*.metainfo.xml`, `*.appdata.xml`).
    pub metainfo_verzeichnisse: Vec<PathBuf>,
    /// Symbolcache-Verzeichnisse (`<basis>/<origin>/<BxH>/`).
    pub symbol_cache_verzeichnisse: Vec<PathBuf>,
    /// Themenverzeichnisse für `stock`-Symbole.
    pub themen_verzeichnisse: Vec<PathBuf>,
    /// Ob `console-application` aufgenommen wird (für die Expertenliste).
    pub mit_konsolenanwendungen: bool,
}

impl Konfiguration {
    /// Die Systempfade aus SCHNITTSTELLEN.md.
    pub fn system() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let mut metainfo = vec![PathBuf::from("/usr/share/metainfo")];
        if let Some(h) = &home {
            metainfo.push(h.join(".local/share/metainfo"));
        }
        Konfiguration {
            katalog_verzeichnisse: vec![
                PathBuf::from("/var/lib/swcatalog/yaml"),
                PathBuf::from("/var/lib/swcatalog/xml"),
                PathBuf::from("/usr/share/swcatalog/yaml"),
                PathBuf::from("/usr/share/swcatalog/xml"),
            ],
            metainfo_verzeichnisse: metainfo,
            symbol_cache_verzeichnisse: symbole::standard_cache_verzeichnisse(),
            themen_verzeichnisse: symbole::standard_themen_verzeichnisse(),
            mit_konsolenanwendungen: true,
        }
    }
}

/// Zusatzdaten je Anwendung, die nicht in `qs_typen::Anwendung` passen (parallel zu `alle()`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Zusatz {
    /// AppStream-Typ (`desktop-application` oder `console-application`).
    pub typ: String,
    /// Origin der Quelle, z. B. `ubuntu-resolute-universe`.
    pub origin: String,
    /// Schlüsselwörter (Deutsch zuerst), Originalschreibung.
    pub schluesselwoerter: Vec<String>,
    /// URL eines `remote`-Symbols (offline nicht geladen).
    pub symbol_fern_url: Option<String>,
    /// `cached`-Einträge aus dem Katalog, für spätere Größen.
    pub symbole_cache: Vec<SymbolDatei>,
    /// Ob Name, Zusammenfassung oder Beschreibung auf Deutsch vorlagen.
    pub deutsch_name: bool,
    pub deutsch_zusammenfassung: bool,
    pub deutsch_beschreibung: bool,
    /// Vorberechnete Suchfelder.
    felder: Suchfelder,
    /// Ob beim Laden eine Symboldatei gefunden wurde.
    symbol_gefunden: bool,
}

/// Ausführlicher Ladebericht (mehr als [`Statistik`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ladebericht {
    pub statistik: Statistik,
    /// Gelesene Einträge inklusive Dubletten über Quellen hinweg.
    pub eintraege_gelesen: usize,
    /// Komponenten je AppStream-Typ (eindeutig nach ID).
    pub je_typ: Vec<(String, usize)>,
    /// Ausgeblendet, weil anderer Typ (addon, font, codec, …).
    pub ausgeblendet_typ: usize,
    /// Ausgeblendet: Anwendung ohne Paketname.
    pub ausgeblendet_ohne_paket: usize,
    /// Ausgeblendet: als Snap/Flatpak-Bundle ausgeliefert.
    pub ausgeblendet_bundle: usize,
    /// Davon Konsolenanwendungen unter den sichtbaren.
    pub konsolenanwendungen: usize,
    /// Anwendungen mit Themensymbol (`stock`), unabhängig davon, ob die Datei da ist.
    pub mit_themensymbol: usize,
    /// Anwendungen, deren Symbol im Katalog-Cache gefunden wurde.
    pub mit_cache_symbol: usize,
    /// Anwendungen je Querencia-Kategorie.
    pub je_kategorie: Vec<(Kategorie, usize)>,
    /// Quellen, die nicht gelesen werden konnten (Pfad, Fehler) – Laden geht weiter.
    pub fehler: Vec<(String, String)>,
    pub dauer: Duration,
}

/// Katalog über die AppStream-Daten des Systems; implementiert [`Katalog`] aus `qs_typen`.
///
/// Bedeutung der [`Statistik`]-Felder:
/// - `komponenten`: eindeutige Komponenten aller Typen in allen Quellen (gleiche ID zählt einmal),
/// - `mit_paket`: sichtbare Anwendungen (`desktop-application`/`console-application` mit Paket,
///   kein Snap/Flatpak-Bundle),
/// - `mit_symbol`: sichtbare Anwendungen, für die lokal eine Symboldatei gefunden wurde
///   (Katalog-Cache oder Themenverzeichnis), nicht nur ein Symbolname,
/// - `deutsch`: sichtbare Anwendungen mit deutscher Zusammenfassung (`de` oder `de_DE`),
/// - `quellen`: gelesene Dateien als `"<Pfad> (<n> Komponenten)"` in Leserreihenfolge.
///
/// Zusätzlich zum Trait: [`AppStreamKatalog::ladebericht`], [`AppStreamKatalog::zusatz`],
/// [`AppStreamKatalog::konsolenanwendungen`].
#[derive(Debug)]
pub struct AppStreamKatalog {
    konfiguration: Konfiguration,
    anwendungen: Vec<Anwendung>,
    zusatz: Vec<Zusatz>,
    index_id: HashMap<String, usize>,
    index_paket: HashMap<String, usize>,
    themen: symbole::ThemenIndex,
    /// Symbolcache-Verzeichnisse, die beim Laden existierten.
    cache_vorhanden: Vec<PathBuf>,
    ladebericht: Ladebericht,
}

impl Default for AppStreamKatalog {
    fn default() -> Self {
        Self::neu()
    }
}

impl AppStreamKatalog {
    /// Katalog mit den Systempfaden; noch leer, bis [`Katalog::laden`] gerufen wurde.
    pub fn neu() -> Self {
        Self::mit_konfiguration(Konfiguration::system())
    }

    /// Katalog mit eigenen Pfaden (Tests, Prüfwerkzeuge).
    pub fn mit_konfiguration(konfiguration: Konfiguration) -> Self {
        AppStreamKatalog {
            konfiguration,
            anwendungen: Vec::new(),
            zusatz: Vec::new(),
            index_id: HashMap::new(),
            index_paket: HashMap::new(),
            themen: symbole::ThemenIndex::default(),
            cache_vorhanden: Vec::new(),
            ladebericht: Ladebericht::default(),
        }
    }

    pub fn konfiguration(&self) -> &Konfiguration {
        &self.konfiguration
    }

    /// Der ausführliche Bericht des letzten Ladens.
    pub fn ladebericht(&self) -> &Ladebericht {
        &self.ladebericht
    }

    /// Zusatzdaten zu einer Anwendung aus diesem Katalog.
    pub fn zusatz(&self, a: &Anwendung) -> Option<&Zusatz> {
        self.position(a).map(|i| &self.zusatz[i])
    }

    /// Ob die Anwendung eine Konsolenanwendung ist (Expertenliste).
    pub fn ist_konsolenanwendung(&self, a: &Anwendung) -> bool {
        self.zusatz(a)
            .is_some_and(|z| z.typ == "console-application")
    }

    /// Alle Konsolenanwendungen, nach Name sortiert.
    pub fn konsolenanwendungen(&self) -> Vec<&Anwendung> {
        self.anwendungen
            .iter()
            .zip(&self.zusatz)
            .filter(|(_, z)| z.typ == "console-application")
            .map(|(a, _)| a)
            .collect()
    }

    fn position(&self, a: &Anwendung) -> Option<usize> {
        self.index_id
            .get(&a.id)
            .copied()
            .filter(|i| self.anwendungen[*i].id == a.id)
    }

    // ---- Laden -------------------------------------------------------------------------------

    fn quelldateien(verzeichnis: &Path, metainfo: bool) -> Vec<PathBuf> {
        let Ok(eintraege) = std::fs::read_dir(verzeichnis) else {
            return Vec::new();
        };
        let mut dateien: Vec<PathBuf> = eintraege
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if metainfo {
                    name.ends_with(".metainfo.xml") || name.ends_with(".appdata.xml")
                } else {
                    name.ends_with(".yml")
                        || name.ends_with(".yml.gz")
                        || name.ends_with(".yaml")
                        || name.ends_with(".yaml.gz")
                        || name.ends_with(".xml")
                        || name.ends_with(".xml.gz")
                }
            })
            .filter(|p| p.is_file())
            .collect();
        // Sortierung so, dass `resolute_main` vor `resolute-updates_main` kommt
        // (`-` → `~` sortiert hinter `_`): spätere Pockets überschreiben frühere.
        dateien.sort_by_cached_key(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .replace('-', "~")
        });
        dateien
    }

    fn lese_quelle(pfad: &Path) -> Result<(Quellkopf, Vec<RohKomponente>), String> {
        let name = pfad.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.contains(".yml") || name.contains(".yaml") {
            dep11::lese_datei(pfad)
        } else {
            xml::lese_datei(pfad)
        }
    }

    fn umwandeln(&self, roh: &RohKomponente) -> (Anwendung, Zusatz) {
        let name = roh.name.bester().unwrap_or(&roh.id).to_string();
        let zusammenfassung = roh.zusammenfassung.bester().unwrap_or("").to_string();
        let beschreibung = roh
            .beschreibung
            .bester()
            .map(beschreibung::markup_zu_text)
            .unwrap_or_default();
        let schluesselwoerter = roh.schluesselwoerter.alle_fuer_suche();

        let mut symbol_gefunden = false;
        let symbol = if let Some(thema) = &roh.symbol_thema {
            symbol_gefunden = self.themen.suchen(thema, 48).is_some();
            Symbol::Thema(thema.clone())
        } else if let Some(p) =
            symbole::cache_pfad(&self.cache_vorhanden, &roh.origin, &roh.symbole_cache, 64)
        {
            symbol_gefunden = true;
            Symbol::Datei(p)
        } else if let Some(l) = roh
            .symbol_lokal
            .as_deref()
            .map(Path::new)
            .filter(|p| p.is_file())
        {
            symbol_gefunden = true;
            Symbol::Datei(l.to_path_buf())
        } else {
            Symbol::Keines
        };

        let felder = Suchfelder {
            name: suche::normalisieren(&name),
            schluesselwoerter: schluesselwoerter
                .iter()
                .map(|k| suche::normalisieren(k))
                .collect(),
            zusammenfassung: suche::normalisieren(&zusammenfassung),
            paketname: suche::normalisieren(roh.paketname.as_deref().unwrap_or("")),
            beschreibung: suche::normalisieren(&beschreibung),
        };
        let anwendung = Anwendung {
            id: roh.id.clone(),
            name,
            zusammenfassung,
            beschreibung,
            paketname: roh.paketname.clone(),
            kategorien: kategorien::zuordnen(&roh.kategorien),
            symbol,
            bildschirmfotos: roh.bildschirmfotos.clone(),
            lizenz: roh.lizenz.clone(),
            homepage: roh.homepage.clone(),
        };
        let zusatz = Zusatz {
            typ: roh.typ.clone(),
            origin: roh.origin.clone(),
            schluesselwoerter,
            symbol_fern_url: roh
                .symbole_fern
                .iter()
                .max_by_key(|s| s.breite)
                .map(|s| s.url.clone()),
            symbole_cache: roh.symbole_cache.clone(),
            deutsch_name: roh.name.deutsch(),
            deutsch_zusammenfassung: roh.zusammenfassung.deutsch(),
            deutsch_beschreibung: roh.beschreibung.deutsch(),
            felder,
            symbol_gefunden,
        };
        (anwendung, zusatz)
    }
}

/// Spätere Quelle gewinnt, aber was ihr fehlt, kommt aus der früheren: Installierte
/// MetaInfo-Dateien haben oft kein `<pkgname>` und kein Katalogsymbol, beschreiben aber dieselbe
/// Komponente wie der Paketkatalog.
fn zusammenfuehren(neu: &mut RohKomponente, alt: &RohKomponente) {
    for (ziel, quelle) in [
        (&mut neu.name, &alt.name),
        (&mut neu.zusammenfassung, &alt.zusammenfassung),
        (&mut neu.beschreibung, &alt.beschreibung),
    ] {
        for (locale, text) in &quelle.0 {
            if ziel.genau(locale).is_none() {
                ziel.0.push((locale.clone(), text.clone()));
            }
        }
    }
    for (locale, woerter) in &alt.schluesselwoerter.0 {
        if !neu.schluesselwoerter.0.iter().any(|(l, _)| l == locale) {
            neu.schluesselwoerter
                .0
                .push((locale.clone(), woerter.clone()));
        }
    }
    if neu.paketname.is_none() {
        neu.paketname = alt.paketname.clone();
    }
    if neu.origin.is_empty() {
        neu.origin = alt.origin.clone();
    }
    if neu.symbole_cache.is_empty() && neu.symbol_thema.is_none() && neu.symbol_lokal.is_none() {
        neu.symbol_thema = alt.symbol_thema.clone();
        neu.symbole_cache = alt.symbole_cache.clone();
    }
    if neu.symbole_fern.is_empty() {
        neu.symbole_fern = alt.symbole_fern.clone();
    }
    if neu.bildschirmfotos.is_empty() {
        neu.bildschirmfotos = alt.bildschirmfotos.clone();
    }
    if neu.kategorien.is_empty() {
        neu.kategorien = alt.kategorien.clone();
    }
    if neu.lizenz.is_none() {
        neu.lizenz = alt.lizenz.clone();
    }
    if neu.homepage.is_none() {
        neu.homepage = alt.homepage.clone();
    }
}

/// Sortierschlüssel für Namen: normalisiert, dann Original, dann ID (stabil und eindeutig).
fn sortierschluessel(a: &Anwendung) -> (String, String, String) {
    (suche::normalisieren(&a.name), a.name.clone(), a.id.clone())
}

impl Katalog for AppStreamKatalog {
    fn laden(&mut self) -> Result<Statistik, String> {
        let start = Instant::now();
        let mut bericht = Ladebericht::default();
        self.themen = symbole::ThemenIndex::aufbauen(&self.konfiguration.themen_verzeichnisse);
        self.cache_vorhanden = self
            .konfiguration
            .symbol_cache_verzeichnisse
            .iter()
            .filter(|p| p.is_dir())
            .cloned()
            .collect();
        let mut quellen: Vec<String> = Vec::new();

        // 1. Alle Quellen lesen; gleiche ID: spätere Quelle gewinnt.
        let mut nach_id: HashMap<String, RohKomponente> = HashMap::new();
        let mut reihenfolge: Vec<String> = Vec::new();
        let mut gelesen = 0usize;
        let mut dateien: Vec<(PathBuf, bool)> = Vec::new();
        for v in &self.konfiguration.katalog_verzeichnisse {
            dateien.extend(Self::quelldateien(v, false).into_iter().map(|p| (p, false)));
        }
        for v in &self.konfiguration.metainfo_verzeichnisse {
            dateien.extend(Self::quelldateien(v, true).into_iter().map(|p| (p, true)));
        }
        for (pfad, _) in &dateien {
            match Self::lese_quelle(pfad) {
                Ok((_kopf, komponenten)) => {
                    gelesen += komponenten.len();
                    quellen.push(format!(
                        "{} ({} Komponenten)",
                        pfad.display(),
                        komponenten.len()
                    ));
                    for mut k in komponenten {
                        match nach_id.get(&k.id) {
                            None => reihenfolge.push(k.id.clone()),
                            Some(alt) => zusammenfuehren(&mut k, alt),
                        }
                        nach_id.insert(k.id.clone(), k);
                    }
                }
                Err(e) => bericht.fehler.push((pfad.display().to_string(), e)),
            }
        }
        if dateien.is_empty() {
            return Err(format!(
                "keine AppStream-Quellen gefunden (gesucht in {:?} und {:?})",
                self.konfiguration.katalog_verzeichnisse, self.konfiguration.metainfo_verzeichnisse
            ));
        }
        bericht.eintraege_gelesen = gelesen;

        // 2. Zählen und filtern.
        let mut je_typ: HashMap<String, usize> = HashMap::new();
        let mut paare: Vec<(Anwendung, Zusatz)> = Vec::new();
        for id in &reihenfolge {
            let roh = &nach_id[id];
            *je_typ.entry(roh.typ.clone()).or_insert(0) += 1;
            if !roh.ist_anwendungstyp()
                || (roh.typ == "console-application" && !self.konfiguration.mit_konsolenanwendungen)
            {
                bericht.ausgeblendet_typ += 1;
                continue;
            }
            if roh.bundle {
                bericht.ausgeblendet_bundle += 1;
                continue;
            }
            if roh.paketname.is_none() {
                bericht.ausgeblendet_ohne_paket += 1;
                continue;
            }
            paare.push(self.umwandeln(roh));
        }
        drop(nach_id);

        // 3. Sortieren und indizieren.
        paare.sort_by_cached_key(|(a, _)| sortierschluessel(a));
        self.anwendungen = Vec::with_capacity(paare.len());
        self.zusatz = Vec::with_capacity(paare.len());
        self.index_id = HashMap::with_capacity(paare.len() * 2);
        self.index_paket = HashMap::with_capacity(paare.len());
        for (i, (a, z)) in paare.into_iter().enumerate() {
            self.index_id.insert(a.id.clone(), i);
            if let Some(kurz) = a.id.strip_suffix(".desktop") {
                self.index_id.entry(kurz.to_string()).or_insert(i);
            }
            if let Some(p) = &a.paketname {
                self.index_paket.entry(p.clone()).or_insert(i);
            }
            self.anwendungen.push(a);
            self.zusatz.push(z);
        }

        // 4. Statistik.
        let mut je_kategorie: Vec<(Kategorie, usize)> =
            Kategorie::alle().iter().map(|k| (*k, 0)).collect();
        for (a, z) in self.anwendungen.iter().zip(&self.zusatz) {
            if z.typ == "console-application" {
                bericht.konsolenanwendungen += 1;
            }
            match &a.symbol {
                Symbol::Thema(_) => bericht.mit_themensymbol += 1,
                Symbol::Datei(_) => bericht.mit_cache_symbol += 1,
                Symbol::Keines => {}
            }
            for k in &a.kategorien {
                if let Some(e) = je_kategorie.iter_mut().find(|(kk, _)| kk == k) {
                    e.1 += 1;
                }
            }
        }
        bericht.je_kategorie = je_kategorie;
        let mut typen: Vec<(String, usize)> = je_typ.into_iter().collect();
        typen.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        bericht.je_typ = typen;
        bericht.statistik = Statistik {
            komponenten: reihenfolge.len(),
            mit_paket: self.anwendungen.len(),
            mit_symbol: self.zusatz.iter().filter(|z| z.symbol_gefunden).count(),
            deutsch: self
                .zusatz
                .iter()
                .filter(|z| z.deutsch_zusammenfassung)
                .count(),
            quellen,
        };
        bericht.dauer = start.elapsed();
        self.ladebericht = bericht;
        Ok(self.ladebericht.statistik.clone())
    }

    fn kategorie(&self, k: Kategorie) -> Vec<&Anwendung> {
        self.anwendungen
            .iter()
            .filter(|a| a.kategorien.contains(&k))
            .collect()
    }

    fn suchen(&self, text: &str) -> Vec<&Anwendung> {
        let normalisiert = suche::normalisieren(text);
        if normalisiert.is_empty() {
            return Vec::new();
        }
        let mut treffer: Vec<(u32, usize)> = self
            .zusatz
            .iter()
            .enumerate()
            .filter_map(|(i, z)| suche::bewerten(&z.felder, &normalisiert).map(|g| (g, i)))
            .collect();
        if treffer.is_empty() {
            // Rückfall: Umlautumschrift auf beiden Seiten weiter falten („buro“ → „Büro“).
            let gefaltet = suche::falten(&normalisiert);
            treffer = self
                .zusatz
                .iter()
                .enumerate()
                .filter_map(|(i, z)| {
                    suche::bewerten(&z.felder.gefaltet(), &gefaltet).map(|g| (g, i))
                })
                .collect();
        }
        // Stabil: bei gleichem Gewicht bleibt die Namensreihenfolge von `alle()` (sort_by ist stabil).
        treffer.sort_by(|a, b| b.0.cmp(&a.0));
        treffer
            .into_iter()
            .take(MAX_TREFFER)
            .map(|(_, i)| &self.anwendungen[i])
            .collect()
    }

    fn nach_paket(&self, paketname: &str) -> Option<&Anwendung> {
        self.index_paket
            .get(paketname)
            .map(|i| &self.anwendungen[*i])
    }

    fn nach_id(&self, id: &str) -> Option<&Anwendung> {
        self.index_id
            .get(id)
            .or_else(|| self.index_id.get(id.strip_suffix(".desktop").unwrap_or(id)))
            .map(|i| &self.anwendungen[*i])
    }

    fn symbol_pfad(&self, a: &Anwendung, groesse: u32) -> Option<PathBuf> {
        match &a.symbol {
            Symbol::Keines => None,
            Symbol::Thema(name) => self.themen.suchen(name, groesse),
            Symbol::Datei(p) => {
                // Im Katalog-Cache die gewünschte Größe suchen, sonst die geladene Datei.
                let z = self.zusatz(a);
                z.and_then(|z| {
                    symbole::cache_pfad(&self.cache_vorhanden, &z.origin, &z.symbole_cache, groesse)
                })
                .or_else(|| p.is_file().then(|| p.clone()))
            }
        }
    }

    fn alle(&self) -> &[Anwendung] {
        &self.anwendungen
    }
}
