#!/bin/bash
# Prüfung gegen die echten AppStream-Daten der VM.
#  1. Integrationstest (Release, #[ignore]) mit Zahlen.
#  2. Ladezeit: fünf Läufe des Beispielwerkzeugs.
#  3. Gegenprobe des YAML-Minimalparsers: dieselben DEP-11-Dateien mit `appstreamcli convert`
#     nach XML wandeln (in einen Prüfordner, nicht in den Systemcache) und beide Wege vergleichen.
# Nur lesend unter /var/lib/swcatalog; schreibt nur in $PRUEF (Standard ~/.cache/querencia-software/pruefung).
set -eu
cd "$(dirname "$0")/.."
PRUEF=${PRUEF:-$HOME/.cache/querencia-software/pruefung}
JOBS=${JOBS:-2}
df -h / | tail -1

echo "== 1. Integrationstest"
nice cargo test --offline -j"$JOBS" --release --test vm -- --ignored --nocapture 2>&1 | grep -vE '^\s+(Compiling|Running)'

echo "== 2. Ladezeit (Release, 5 Läufe)"
nice cargo build --offline -j"$JOBS" --release --example katalog-suche 2>&1 | tail -1
E=target/release/examples/katalog-suche
for i in 1 2 3 4 5; do "$E" | sed -n 1p; done

echo "== 3. Gegenprobe YAML-Minimalparser gegen appstreamcli convert"
rm -rf "$PRUEF/xml"
mkdir -p "$PRUEF/xml"
start=$(date +%s.%N)
for f in /var/lib/swcatalog/yaml/*.yml.gz; do
  name=$(basename "$f" .yml.gz)
  appstreamcli convert "$f" "$PRUEF/xml/$name.xml" >/dev/null 2>&1 || echo "convert fehlgeschlagen: $f"
done
ende=$(date +%s.%N)
echo "appstreamcli convert, 16 Dateien: $(echo "$ende - $start" | bc) s, $(du -sh "$PRUEF/xml" | cut -f1)"
QS_KATALOG_VERZEICHNIS=/var/lib/swcatalog/yaml "$E" > "$PRUEF/stat-yaml.txt"
QS_KATALOG_VERZEICHNIS="$PRUEF/xml" "$E" > "$PRUEF/stat-xml.txt"
QS_KATALOG_VERZEICHNIS=/var/lib/swcatalog/yaml "$E" suche e > "$PRUEF/suche-yaml.txt"
QS_KATALOG_VERZEICHNIS="$PRUEF/xml" "$E" suche e > "$PRUEF/suche-xml.txt"
echo "-- YAML-Weg: $(head -1 "$PRUEF/stat-yaml.txt")   XML-Weg: $(head -1 "$PRUEF/stat-xml.txt")"
if diff <(tail -n +3 "$PRUEF/stat-yaml.txt") <(tail -n +3 "$PRUEF/stat-xml.txt") 2>/dev/null; then
  echo "Statistik identisch (ohne Ladezeit und Quellzeile)"
else
  echo "Statistik weicht ab (siehe oben)"
fi
QS_KATALOG_VERZEICHNIS=/var/lib/swcatalog/yaml "$E" dump > "$PRUEF/dump-yaml.txt"
QS_KATALOG_VERZEICHNIS="$PRUEF/xml" "$E" dump > "$PRUEF/dump-xml.txt"
if cmp -s "$PRUEF/dump-yaml.txt" "$PRUEF/dump-xml.txt"; then
  echo "Alle $(wc -l < "$PRUEF/dump-yaml.txt") Anwendungen mit allen Feldern identisch"
else
  echo "Abweichende Anwendungen: $(diff "$PRUEF/dump-yaml.txt" "$PRUEF/dump-xml.txt" | grep -c '^<')"
  diff "$PRUEF/dump-yaml.txt" "$PRUEF/dump-xml.txt" | head -6 | cut -c1-400
fi
if cmp -s <(sed '$d' "$PRUEF/suche-yaml.txt") <(sed '$d' "$PRUEF/suche-xml.txt"); then
  echo "Suche 'e' (200 Treffer, Name/Paket/Zusammenfassung/Symbol): identisch"
else
  echo "Suche 'e' weicht ab:"; diff "$PRUEF/suche-yaml.txt" "$PRUEF/suche-xml.txt" | head -20
fi
