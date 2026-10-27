# Änderungsprotokoll

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [0.1.1] – 2026-10-27

### Behoben
- Programme erschienen doppelt (z. B. Remmina): Eine Komponenten-ID mit der Endung `.desktop`
  gilt jetzt nach der AppStream-Regel als dieselbe Komponente wie die ID ohne Endung. Die ID wird
  beim Einlesen ohne Endung gespeichert, sonst unverändert (Groß-/Kleinschreibung bleibt); bei
  gleicher ID gewinnt weiter die später gelesene Quelle. `nach_id` findet beide Schreibweisen.
- Ein späterer Eintrag vom Typ `generic` mit derselben ID verdrängt keine Anwendung mehr
  (Subtitle Editor).
- Installierte MetaInfo-Dateien mit dem alten Typ `desktop` gelten als `desktop-application`
  (GIMP fehlte im Katalog, sobald es installiert war).

### Hinzugefügt
- `katalog-suche doppel`: zählt Kennungen mit beiden Schreibweisen in den Quellen und doppelte
  Anwendungen im Katalog.

## [0.1.0] – 2026-10-18

Erster Stand (Tag `katalog/0.1.0`).

### Hinzugefügt
- AppStream-Leser für DEP-11 (YAML) und XML, Zusammenführen mit installierten MetaInfo-Dateien.
- Querencia-Kategorien, gewichtete und umlauttolerante Suche, Symbolindex.
- Beispielwerkzeug `katalog-suche`, Tests mit Fixtures und gegen echte Systemdaten.
