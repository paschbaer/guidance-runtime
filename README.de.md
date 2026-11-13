# Guidance Runtime Service

Windows-Dienst, CLI, Tray-Anwendung, MCP-Workload-Supervisor und GitHub-Release-Updater in Rust. Die englische Anleitung steht in [README.md](README.md).

## Build

```powershell
.\scripts\build.ps1
```

Die Release-Binaries liegen danach unter `target\release`: `guidance-service.exe`, `guidance.exe`, `guidance-tray.exe` und `guidance-updater.exe`.

## Konfiguration und Installation

```powershell
Copy-Item .\config\runtime.example.toml .\config\runtime.toml
Copy-Item .\config\updates.example.toml .\config\updates.toml
.\scripts\install-dev.ps1
```

In `updates.toml` werden GitHub-Repository und Update-Channel festgelegt:

```toml
repository = "paschbaer/guidance-runtime"
channel = "stable" # stable oder beta
```

Diese flache Form bleibt ein einzelnes Ziel namens `guidance-runtime` und behält ihre historische Ausgabe. Sie bleibt auch beim Ändern des Channels flach. Für mehrere Produkte gibt es die `[[targets]]`-Form unter [Updates](#updates).

## Git-Flow und automatische Releases

```text
feature/* -> develop -> Beta-Release
                     -> Pull Request -> main -> Stable-Release
```

- Ein Push nach `develop` erzeugt ein GitHub-Prerelease mit dem Tag `v1.3.0-beta.<run-number>`.
- Ein Push nach `main` erzeugt ein Stable-Release mit dem Tag `v1.3.<run-number>` und markiert es als Latest.
- Nur Änderungen unter `crates/**`, `config/**`, `scripts/**`, an den Cargo-Dateien, an `rust-toolchain.toml` oder am Release-Workflow lösen ein Release aus.
- `[skip release]` in der Head-Commit-Nachricht unterdrückt den Release-Lauf.
- Der Workflow prüft doppelte Tags und Releases zweimal: vor dem Build und unmittelbar vor der Veröffentlichung. Vorhandene Tags oder Releases werden nie überschrieben.

Empfohlene Repository-Regeln:

1. `main` schützen und Pull Requests aus `develop` verlangen.
2. Den CI-Workflow und mindestens eine Freigabe voraussetzen.
3. Force-Pushes und das Löschen von Tags deaktivieren.
4. Das GitHub Environment `stable` mit Required Reviewers schützen. `beta` kann automatisch veröffentlichen.

Die Versionslinie wird über `VERSION_MAJOR` und `VERSION_MINOR` in `.github/workflows/release.yml` gesteuert. Beim Start einer neuen Release-Linie müssen diese Werte bewusst angepasst werden.

## Updates

```powershell
guidance update channel [--target NAME]
guidance update channel beta [--target NAME]
guidance update check [--format text|json] [--target NAME]
guidance update install [--target NAME] --yes
```

`--target` beschränkt Check oder Installation auf ein konfiguriertes Ziel. Ohne Angabe prüft ein Check jedes Ziel, und eine Installation wendet jedes Ziel mit verfügbarem Update an. `--yes` ist für jede Installation Pflicht.

### Mehrere Update-Ziele

Ein einzelnes Produkt kann die flache Form oben behalten. Für mehrere Produkte mit einem Updater werden sie als `[[targets]]` deklariert (die beiden Formen nicht mischen):

```toml
[[targets]]
name = "guidance-runtime"
repository = "paschbaer/guidance-runtime"
visibility = "public"
channel = "stable"
install_dir = "C:/Program Files/Guidance Runtime"
service_name = "GuidanceRuntime"

[[targets]]
name = "guidance"
repository = "paschbaer/guidance-dist"
visibility = "private"
credential_entry = "guidance-update"
channel = "stable"
install_dir = "C:/Program Files/Guidance"
```

Zwei Ziele dürfen kein `install_dir` teilen, und kein `install_dir` darf dem Rollback-Verzeichnis eines anderen Ziels entsprechen. `install_dir` ist in dieser Form Pflicht; nur die flache Form behält ihren historischen Standardwert. Ein Ziel namens `guidance-runtime` behält den Dienst `GuidanceRuntime` auch ohne `service_name`; jedes andere Ziel ohne `service_name` wird ohne Dienststeuerung installiert.

### Private Quellen und Zugangsdaten

Ein privates Ziel nennt in `credential_entry` den Namen eines Eintrags im Credential-Store. Das Geheimnis selbst steht nie in der Konfiguration. Der Eintrag wird einmal pro Benutzer bereitgestellt:

| Plattform | Befehl |
|---|---|
| Windows | Anmeldeinformationsverwaltung, generische Anmeldeinformation, Ziel `guidance:update:<entry>` |
| macOS | `security add-generic-password -s guidance-update -a <entry> -w` |
| Linux | `secret-tool store --label='Guidance' service guidance-update username <entry>` |

Das Token darf **nur Lesezugriff auf die Release-Artefakte dieses Repositorys** haben. Es darf keinen Zugriff auf Quellcode-Repositories und keinen Schreibzugriff gewähren. Fehlt der Eintrag, meldet die Ausgabe `no access to update source` und nennt den bereitzustellenden Eintrag.

### Ausgabe und Exit-Status

Ein Check gibt eine Zeile pro Ziel aus, plus eine eingerückte Detailzeile, wenn der Zustand eine handlungsrelevante Ursache hat (etwa einen nicht bereitgestellten Eintrag). `--format json` gibt stattdessen ein Dokument mit `spec_version`, `checked_at`, `exit_status` und einem `targets`-Array aus; mit `--target` enthält das Array genau einen Eintrag. Die Zustände sind `update_available`, `up_to_date`, `not_installed`, `inaccessible`, `network_error`, `invalid_artifact` und `credential_store_unavailable`.

Exit-Status: `0` sauber, `1` Konfigurations- oder Auswahlfehler, die der Updater selbst erkennt, `2` mindestens ein Ziel fehlgeschlagen. Ein Kommandozeilen-Nutzungsfehler (etwa ein ungültiger `--format`-Wert) endet mit `1`: es wurde nichts ausgeführt. Exit `2` ist Ziel-Fehlschlägen vorbehalten und wird nie für Nutzungsfehler verwendet. Die historische flache Form behält ihre bisherige Wortwahl und Exit `1`, wenn der Quellzugriff fehlschlägt.

### Installation und installierte Version

Der Updater lädt das ZIP, das zu Plattform und Architektur passt, zusammen mit der SHA-256-Datei, prüft das Archiv, sichert die Installation des Ziels, ersetzt Dateien, startet den Dienst des Ziels neu, falls es einen hat, und stellt das Backup wieder her, wenn etwas fehlschlägt. Das Archiv bestimmt seinen Inhalt: Ein `bin/`-Verzeichnis beschränkt den Austausch auf dessen Inhalt, sonst wird das Archiv-Wurzelverzeichnis installiert. Kopieren überschreibt Dateien, löscht aber nie das Installationsverzeichnis.

Die installierte Version steht in `install.json` im Installationsverzeichnis des Ziels. Ein fehlender, unlesbarer oder zu einem anderen Ziel gehörender Marker wird als `not installed` gemeldet. Der Marker wird erst nach einer erfolgreichen Installation geschrieben, damit eine fehlgeschlagene Installation nie die neue Version beansprucht. Eine Übergangsregel schützt Installationen aus der Zeit vor dem Marker: Nur für `guidance-runtime` gilt ein fehlender Marker zusammen mit `guidance-service.exe` im Installationsverzeichnis als installiert in der laufenden Version.

`guidance update install` startet einen abgekoppelten Updater und endet mit `0`, sobald dieser läuft; das Ergebnis pro Ziel und der Exit-Status des Updaters (`0` oder `2`) entstehen in diesem Prozess, nachdem die CLI beendet ist.

Die Installation ist nur unter Windows implementiert. macOS und Linux können nach Updates suchen; eine Installation dort meldet, dass sie nur unter Windows unterstützt wird. Die Release-Matrix veröffentlicht weiterhin nur Windows-Archive, bis Nicht-Windows-Artefakte veröffentlicht werden.

Das Tray-Menü zeigt den aktuell konfigurierten Update-Channel und enthält **Check for updates**.
