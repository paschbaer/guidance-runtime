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
repository = "YOUR-ORG/guidance-runtime"
channel = "stable" # stable oder beta
```

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
guidance update channel
guidance update channel beta
guidance update check
guidance update install --yes
```

Der Updater lädt das passende Windows-x64-ZIP und die SHA-256-Datei, prüft das Archiv, stoppt `GuidanceRuntime`, sichert die Installation, ersetzt die Binaries und startet den Dienst neu. Schlägt Austausch oder Neustart fehl, wird das Backup wiederhergestellt.

Das Tray-Menü zeigt den aktuell konfigurierten Update-Channel und enthält **Check for updates**.
