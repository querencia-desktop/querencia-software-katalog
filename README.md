# qs_katalog – AppStream-Katalog für Querencia Software

AppStream-Katalog für Querencia Software: liest die AppStream-Daten des Systems, ordnet Kategorien zu, sucht und löst Symbole auf (`qs_katalog`). Teil von
[Querencia Desktop](https://github.com/querencia-desktop/querencia) – einer
klassischen Desktopumgebung im GNOME-2-Stil für Wayland (labwc, Rust, GTK 4).

*English summary:* AppStream catalogue of Querencia Software: reads the system's AppStream data, maps categories, searches and resolves icons (`qs_katalog`). Part of Querencia Desktop. Alpha software; documentation is in German.

> **Alpha.** Version 0.1.0 (Tag `katalog/0.1.0`). Keine Sicherheits- oder Datenverlustgarantie.

## Zusammenspiel der Repos

Die Bausteine von Querencia Software liegen in vier Repos nebeneinander und
binden sich über Pfadabhängigkeiten ein. Zum Bauen und Prüfen alle vier in
denselben Ordner klonen:

```bash
for r in querencia-software querencia-software-paketkit querencia-software-katalog querencia-software-transaktion; do
  git clone https://github.com/querencia-desktop/$r.git
done
```

Die Kennungen SW-1 bis SW-4 in Kommentaren bezeichnen die Teilprojekte:
SW-1 `querencia-software-paketkit`, SW-2 `querencia-software-katalog`,
SW-3 die Oberfläche in `querencia-software`, SW-4 `querencia-software-transaktion`.
Die gemeinsame Schnittstelle beschreibt `docs/SCHNITTSTELLEN.md` in
`querencia-software`.

Bibliothek (SW-2) für die Oberfläche „Querencia Software“ (SW-3). Liest die AppStream-Daten des
Systems, zeigt nur installierbare Anwendungen mit Debian-Paket, bildet die Freedesktop-Kategorien
auf die zehn Querencia-Kategorien ab, sucht gewichtet und umlauttolerant und löst Symbole auf.
Schnittstelle: das Trait `qs_typen::Katalog` aus `docs/SCHNITTSTELLEN.md` in `querencia-software`,
implementiert von `qs_katalog::AppStreamKatalog`.

```rust
use qs_katalog::{AppStreamKatalog, Katalog, Kategorie};

let mut katalog = AppStreamKatalog::neu();          // Systempfade
let statistik = katalog.laden()?;                    // ≈ 0,3 s auf der Entwicklungsmaschine, im Hintergrund-Thread rufen
let buero = katalog.kategorie(Kategorie::Buero);     // nach Name sortiert
let treffer = katalog.suchen("büro");                // höchstens 200, nach Gewicht
let gedit = katalog.nach_paket("gedit");
let pfad = gedit.and_then(|a| katalog.symbol_pfad(a, 48));
```

## Quellen (in dieser Reihenfolge, spätere gewinnen bei gleicher ID)

1. `/var/lib/swcatalog/yaml`, `/var/lib/swcatalog/xml`, `/usr/share/swcatalog/yaml`, `/usr/share/swcatalog/xml`
   (`*.yml[.gz]`, `*.xml[.gz]`; innerhalb eines Ordners so sortiert, dass `resolute-updates` nach `resolute` kommt)
2. `/usr/share/metainfo/*.{metainfo,appdata}.xml`, `~/.local/share/metainfo/*.xml`

Fehlt einer späteren Quelle etwas (typisch: installierte MetaInfo ohne `<pkgname>`, ohne Symbol,
ohne Beschreibung), wird es aus der früheren übernommen. Eine unlesbare Datei landet in
`ladebericht().fehler`, das Laden geht weiter.

## Entscheidungen

- **DEP-11-YAML:** eigener Minimalparser (`src/yaml.rs`), kein `appstreamcli convert`, kein Cache.
  Begründung und Messung im Bericht .
- **Sichtbar** sind `desktop-application` und `console-application` mit Paketname und ohne
  Snap/Flatpak-Bundle. Konsolenanwendungen: `AppStreamKatalog::konsolenanwendungen()` für die
  Expertenliste; abschaltbar über `Konfiguration::mit_konsolenanwendungen`.
- **Locale:** `de` → `de_DE` → `C` (dann `en`, `en_GB`, `en_US` als Notnagel) für Name,
  Zusammenfassung, Beschreibung; Schlüsselwörter aller dieser Fassungen werden vereinigt.
- **Beschreibung:** reiner Text. Absätze durch Leerzeile, Listenpunkte „• “ (geordnet „1. “),
  `em`/`code` nur Text, Elemente mit eigenem `xml:lang` mitten in einer Fassung werden verworfen.
- **Kategorien:** Tabelle `kategorien::ABBILDUNG` (≈ 140 Freedesktop-Namen), Mehrfachzuordnung,
  Rest „Sonstiges“. Toolkit-Marken (`GTK`, `Qt`, `KDE` …) zählen nicht.
- **Suche:** Gewichte je Suchwort Name-Präfix 64 > Name enthält 32 > Schlüsselwort 16 >
  Zusammenfassung 8 > Paketname 4 > Beschreibung 2, +128 bei genauem Namen; alle Wörter müssen
  treffen; Vergleich klein, `ä`→`ae`, Akzente weg; ohne Treffer Rückfall mit `ae`→`a` (findet
  „buro“). Stabil sortiert, Gleichstand in Namensreihenfolge.
- **Symbole:** `Symbol::Thema(name)` für `stock`; sonst `Symbol::Datei` aus dem Katalog-Cache
  `<cache>/<origin>/<B>x<H>/` (`/var/lib/swcatalog/icons`, `/var/cache/swcatalog/icons`, …) oder
  `local`; sonst `Symbol::Keines`. `remote` wird nie geladen, die URL steht in
  `zusatz(a).symbol_fern_url`. `symbol_pfad(a, 48|64)` sucht über einen beim Laden gebauten
  Index von hicolor/Adwaita/gnome/`~/.local/share/icons/hicolor` und `/usr/share/pixmaps`.

## Werkzeug

```bash
cargo run --offline --release --example katalog-suche                       # Statistik
cargo run --offline --release --example katalog-suche -- suche texteditor
cargo run --offline --release --example katalog-suche -- kategorie Büro
cargo run --offline --release --example katalog-suche -- paket gedit
cargo run --offline --release --example katalog-suche -- id org.gnome.gedit.desktop
cargo run --offline --release --example katalog-suche -- dump               # alle Felder, für Vergleiche
```

`QS_KATALOG_VERZEICHNIS=<ordner>` liest statt der Systempfade nur diesen Ordner.

## Prüfen

```bash
nice cargo test --offline -j2                         # 45 Tests mit Fixtures (tests/fixtures)
nice cargo clippy --offline -j2 --all-targets -- -D warnings
skripte/vm-pruefung.sh                                # echte VM: Integrationstest, Ladezeit, Gegenprobe
```

`skripte/vm-pruefung.sh` liest nur unter `/var/lib/swcatalog` und schreibt nur nach
`~/.cache/querencia-software/pruefung/` (wandelt dort zur Gegenprobe alle DEP-11-Dateien mit
`appstreamcli convert` nach XML und vergleicht beide Einlesewege Feld für Feld).

## Abhängigkeiten

`qs_typen` (Pfad `../querencia-software-paketkit/qs_typen`, SW-1), `roxmltree 0.21.1`,
`flate2 1.1.10`. Alles `--offline` aus dem Cargo-Cache.

## Lizenz

GPL-3.0-or-later, siehe [LICENSE](LICENSE). Copyright 2026 Querencia Desktop
contributors; Urheberhinweis in [COPYRIGHT](COPYRIGHT). Der Wechsel der Bausteine
auf „MIT OR Apache-2.0“ ist geplant, aber noch nicht vollzogen.

Name und Logo „Querencia“ sind keine freien Inhalte: [MARKE.md](MARKE.md).

## Fehler melden

Fehler, Wünsche und Fragen zu Querencia gehen an die gemeinsame Meldestelle:
<https://github.com/querencia-desktop/querencia/issues/new/choose>.
Sicherheitslücken bitte nicht öffentlich melden, siehe [SECURITY.md](SECURITY.md).
