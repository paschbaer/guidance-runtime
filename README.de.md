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

## Initiale Installation und Konfiguration

Dieser Abschnitt beschreibt die Erstinstallation (Greenfield) auf einem leeren Windows-System. Das Upgrade einer bestehenden Installation ist unter [Updates](#updates) beschrieben. Eine Rust-Toolchain ist auf Zielsystemen **nicht** erforderlich — sie wird nur beim Installieren aus dem Quellcode benötigt.

### Voraussetzungen

- Windows mit einer PowerShell als Administrator (Dienst und Update-Installation sind nur unter Windows implementiert).
- Netzwerkzugriff auf GitHub Releases für die konfigurierten Update-Quellen.
- Nur beim Installieren aus dem Quellcode: eine Rust-Toolchain; `rust-toolchain.toml` pinned Version und Komponenten, `cargo` übernimmt sie automatisch.

### 1. Binaries beziehen

- **Aus einem Release (Zielsysteme):** `guidance-runtime-<version>-<arch>.zip` zusammen mit der SHA-256-Datei von der GitHub-Releases-Seite des Repositorys herunterladen und die Prüfsumme verifizieren. Das Archiv enthält `bin/` mit den vier Executables und `config/` mit den TOML-Beispieldateien.
- **Aus dem Quellcode (Entwicklung):** `scripts\build.ps1` ausführen; die Binaries liegen danach unter `target\release`.

### 2. Runtime installieren

`guidance-service.exe`, `guidance.exe`, `guidance-tray.exe` und `guidance-updater.exe` nach `C:\Program Files\Guidance Runtime` kopieren, `C:\ProgramData\Guidance Runtime\config` und `C:\ProgramData\Guidance Runtime\logs` erstellen und dort `runtime.toml` und `updates.toml` ablegen (Ausgang: die `*.example.toml`-Dateien). Ein späteres Selbstupdate ersetzt genau diese vier Binaries im Installationsverzeichnis — das Layout passt also zu dem, was der Updater pflegt.

- **Aus einem Repository-Checkout:** `scripts\install-dev.ps1` ausführen (als Administrator) — es erledigt all das und registriert den Dienst; `scripts\uninstall-dev.ps1` entfernt die Installation wieder.
- **Aus einem Release-Archiv:** den Dienst manuell registrieren und starten:

Einfache Anführungszeichen überleben `sc.exe` nicht: es parst seine eigene Kommandozeile und verbraucht die Quotes, und die Maskierung, die das überlebt, unterscheidet sich zwischen `cmd.exe`, Windows PowerShell und PowerShell 7. `New-Service` umgeht das völlig — es übergibt den Binary-Pfad direkt an den Dienst-Manager, sodass die quotierte Kommandozeile unverändert im `ImagePath` des Dienstes landet:

```powershell
New-Service -Name GuidanceRuntime -BinaryPathName '"C:\Program Files\Guidance Runtime\guidance-service.exe" --config "C:\ProgramData\Guidance Runtime\config\runtime.toml"' -StartupType Automatic -DisplayName 'Guidance Runtime Service'
sc.exe failure GuidanceRuntime reset= 86400 actions= restart/5000/restart/15000/restart/60000
sc.exe start GuidanceRuntime
```

In `cmd.exe` lautet das Äquivalent `sc.exe create GuidanceRuntime binPath= "\"C:\Program Files\Guidance Runtime\guidance-service.exe\" --config \"C:\ProgramData\Guidance Runtime\config\runtime.toml\"" start= auto DisplayName= "Guidance Runtime Service". Prüfen mit `sc.exe qc GuidanceRuntime`: `BINARY_PATH_NAME` muss die Anführungszeichen enthalten.

Die Workloads und das Log-Verzeichnis werden in `C:\ProgramData\Guidance Runtime\config\runtime.toml` gepflegt (`log_dir`, `[[workloads]]`); nach Änderungen den Dienst neu starten.

### 3. Self-Updater konfigurieren

Die Binaries lesen die Update-Konfiguration standardmäßig aus `C:\ProgramData\Guidance Runtime\config\updates.toml` — dort muss alles Eingetragene stehen, nicht nur in der Kopie im Repository.

- **Selbstupdate von `guidance-runtime`:** flache Form mit `repository` und `channel` (`stable` oder `beta`) wie unter [Konfiguration und Installation](#konfiguration-und-installation) gezeigt.
- **`guidance` installierbar machen:** dafür ist die `[[targets]]`-Form nötig (die flachen Schlüssel löschen, die Formen nicht mischen). Beispiel siehe [Mehrere Update-Ziele](#mehrere-update-ziele): Ziel `guidance` mit `repository = "paschbaer/guidance-dist"`, `visibility = "private"`, `credential_entry`, `channel` und `install_dir` (Pflicht, z. B. `C:/Program Files/Guidance`).
- **Privates Ziel bereitstellen:** einmal pro Benutzer eine generische Anmeldeinformation in der Windows-Anmeldeinformationsverwaltung mit dem Ziel `guidance:update:guidance-update` anlegen; das Token darf nur Lesezugriff auf die Release-Artefakte dieses Repositorys haben. Details unter [Private Quellen und Zugangsdaten](#private-quellen-und-zugangsdaten).
- **`guidance` ausführbar machen:** ein Ziel ohne `service_name` wird ohne Dienststeuerung installiert; das Installationsverzeichnis (z. B. `C:\Program Files\Guidance`) in `PATH` aufnehmen, damit `guidance` von überall aufrufbar ist.

### 4. `guidance` erstmals installieren und Installation prüfen

Der Updater aktualisiert bestehende Installationen: Ein Ziel ohne `install.json`-Marker in seinem `install_dir` wird als `not_installed` gemeldet und von `guidance update install` übersprungen. Die **Erstinstallation von `guidance` ist daher manuell**, aus dem Release-Archiv von `paschbaer/guidance-dist`:

```powershell
# ZIP und SHA-256 von https://github.com/paschbaer/guidance-dist/releases
# herunterladen (GitHub-Anmeldung nötig), Prüfsumme verifizieren, dann:
Expand-Archive .\guidance-dist-1.2.0-windows-x64.zip -DestinationPath .
Copy-Item .\guidance-dist-1.2.0-windows-x64\* "C:\Program Files\Guidance\" -Recurse -Force
```

(Archiv- und Asset-Namen sind Beispiele — die tatsächlichen Namen stehen beim Release. Enthält das Archiv ein `bin/`-Verzeichnis, dessen Inhalt nach `C:\Program Files\Guidance` kopieren.)

Anschließend den Versionsmarker schreiben, auf den der Updater sich stützt — `C:\Program Files\Guidance\install.json`:

```json
{
  "target": "guidance",
  "version": "1.2.0",
  "installed_at": "2026-11-13T10:00:00Z"
}
```

`target` muss dem Zielnamen aus `updates.toml` entsprechen, `version` der installierten Release (Semver). Von da an verwaltet der Updater das Produkt:

```powershell
& "C:\Program Files\Guidance Runtime\guidance.exe" update check   # meldet guidance als up to date
& "C:\Program Files\Guidance Runtime\guidance.exe" update install --target guidance --yes
```

Bei einer Entwicklungsinstallation funktioniert `target\release\guidance.exe` genauso.

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

### Channels

Es gibt zwei Channels, je Ziel über den Schlüssel `channel` gewählt. Ein fehlender Schlüssel bedeutet `stable`; die Werte sind case-insensitiv, und alles andere als `stable` oder `beta` wird abgelehnt (`channel must be stable or beta`).

- **`stable`** (Standard) — bietet ausschließlich veröffentlichte Releases: kein GitHub-Prerelease-Flag und kein Prerelease-Suffix im Tag (zum Beispiel `v1.3.7`). Diese entstehen aus Pushes nach `main`.
- **`beta`** — bietet zusätzlich Prereleases, deren Prerelease-Identifier mit `beta.` beginnt, also die Form, die der Release-Workflow aus `develop` veröffentlicht (zum Beispiel `v1.3.0-beta.42`). Andere Prerelease-Labels wie `alpha.` oder `rc.` werden nie angeboten, und Drafts werden auf keinem Channel angeboten.

Angeboten wird immer nur, was über der installierten Version liegt. `guidance update channel beta [--target NAME]` schreibt die Konfiguration an Ort und Stelle um; eine flache Datei bleibt flach, eine `[[targets]]`-Datei bleibt ein Array.

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
| Windows | Anmeldeinformationsverwaltung, generische Anmeldeinformation, Ziel `guidance:update:<entry>` — Details unten |
| macOS | `security add-generic-password -s guidance-update -a <entry> -w` |
| Linux | `secret-tool store --label='Guidance' service guidance-update username <entry>` |

Das Token darf **nur Lesezugriff auf die Release-Artefakte dieses Repositorys** haben. Es darf keinen Zugriff auf Quellcode-Repositories und keinen Schreibzugriff gewähren. Fehlt der Eintrag, meldet die Ausgabe `no access to update source` und nennt den bereitzustellenden Eintrag.

#### Windows Schritt für Schritt am Beispiel `guidance` (`paschbaer/guidance-dist`)

Ausgangspunkt ist dieser Eintrag in `C:\ProgramData\Guidance Runtime\config\updates.toml`:

```toml
[[targets]]
name = "guidance"
repository = "paschbaer/guidance-dist"
visibility = "private"
credential_entry = "guidance-update"
channel = "stable"
install_dir = "C:/Program Files/Guidance"
```

Daraus ergibt sich der Zielname der Anmeldeinformation: fester Präfix `guidance:update:` plus der Wert aus `credential_entry` → **`guidance:update:guidance-update`**. Genau dieser String wird unten Feld für Feld eingetragen. Die Versionsnummern in den Beispielausgaben sind Beispiele.

**Voraussetzung — Zugriff auf das Repository (das beabsichtigte Modell).** Jeder Nutzer arbeitet mit seinem eigenen GitHub-Konto: Der Eigentümer lädt die Konten explizit zum Repository ein (im Repository: **Settings → Collaborators → Add people**, Rolle `read`) oder, bei Organisations-Repositories, über ein Team mit Lesezugriff. Weil `guidance-dist` ein reines Dist-Repository ist (nur Release-Artefakte, kein Quellcode), reicht `read` vollständig — es gewährt genau den Zugriff auf die Releases und nichts weiter. Das Token kann nur Rechte ausüben, die sein GitHub-Konto selbst hat; ein Token eines Kontos ohne Repo-Zugriff erzeugt später dieselbe Meldung `no access to update source` wie ein fehlender Eintrag — der Browser-Test unterscheidet die beiden Fälle (404 = kein Zugriff). Jeder Nutzer erstellt und hinterlegt sein **eigenes** Token; geteilte Tokens nicht verwenden — sie sind einzeln nicht widerrufbar, rotieren alle gleichzeitig und sind nicht zuordenbar.

**Schritt 0 — Zustand ohne Eintrag ansehen.** `guidance update check` meldet (Rendering wie im Code):

```text
guidance-runtime  up to date (1.3.7, stable)
guidance  no access to update source
  no credential named 'guidance-update' is provisioned on this machine; provision it as the generic Credential Manager credential 'guidance:update:guidance-update'
```

Die eingerückte Zeile nennt bereits den anzulegenden Zielnamen.

**Schritt 1 — Token bei GitHub erstellen.** github.com → Profilsymbol → **Settings** → **Developer settings** → **Personal access tokens** → **Fine-grained tokens** → **Generate new token**:

- *Token name:* `guidance-updates`; *Expiration:* nach eigener Richtlinie (z. B. 90 Tage).
- *Resource owner:* `paschbaer`.
- *Repository access:* **Only select repositories** → `paschbaer/guidance-dist`.
- *Permissions → Repository permissions:* **Contents: Read-only** (Releases und ihre Assets fallen darunter; `Metadata` wird automatisch auf Read-only gesetzt). Alles andere bleibt ohne Zugriff.
- **Generate token** → das Token (`github_pat_…`) sofort kopieren; es wird nur einmal angezeigt.

**Schritt 2a — Eintrag über die GUI anlegen.** Systemsteuerung → Benutzerkonten → **Anmeldeinformationsverwaltung** → Registerkarte **Windows-Anmeldeinformationen** → **Generische Anmeldeinformation hinzufügen**:

| Feld | Wert |
|---|---|
| Internetadresse oder Netzwerkadresse | `guidance:update:guidance-update` |
| Benutzername | `paschbaer` (frei wählbar — der Updater liest nur das Kennwort-Feld) |
| Kennwort | das Token `github_pat_…` einfügen |

Mit **Speichern** ablegen. Feldbezeichnungen können je nach Windows-Sprachversion leicht abweichen.

**Schritt 2b — oder per `cmdkey`:**

```powershell
cmdkey /generic:guidance:update:guidance-update /user:paschbaer /pass
```

`/pass` ohne Wert fragt das Token ab, ohne es anzuzeigen — es landet weder im Shell-Verlauf noch in der Prozessliste. Die Form `/pass:<token>` hinterlässt das Token im Klartext im Verlauf und ist zu vermeiden. Alternativ bietet der WiX-MSI-Installer im Abschlussdialog eine Checkbox (Standard: aus), die genau diese `cmdkey`-Abfrage im eigenen Benutzerkontext startet; das MSI selbst enthält kein Token.

**Schritt 3 — Eintrag prüfen:**

```powershell
cmdkey /list | findstr guidance:update
```

Erwartet (Beschriftung sprachabhängig; entscheidend ist die Ziel-Zeile):

```text
    Ziel: LegacyGeneric:target=guidance:update:guidance-update
    Typ: Generisch
```

**Schritt 4 — erneut prüfen.** `guidance update check` meldet jetzt:

```text
guidance  not installed
```

Das heißt: der Zugriff klappt, aber `guidance` ist noch nicht installiert (es fehlt der Marker `install.json`) — die Erstinstallation geschieht manuell, siehe Schritt 4 des Installationsabschnitts. Danach meldet der Check `guidance  up to date (1.2.0, stable)` bzw. bei neuerem Release `guidance  update available: 1.2.0 -> 1.3.0 (stable)`, und `guidance update install --target guidance --yes` übernimmt alle folgenden Updates.

Hinweise: Der Eintrag gilt **pro Windows-Konto** — das Konto, das ihn anlegt, muss auch `guidance update` ausführen. Der Typ muss **generisch** sein (keine Domänen-/Web-Anmeldeinformation). Ein leerer Kennwort-Wert gilt als „nicht vorhanden“. Meldet der Check trotz Eintrags weiterhin `no access to update source`, hat das Token keinen Lesezugriff auf die Releases (Token neu ausstellen); `credential_store_unavailable` bedeutet, dass der Store selbst nicht lesbar ist.

### Ausgabe und Exit-Status

Ein Check gibt eine Zeile pro Ziel aus, plus eine eingerückte Detailzeile, wenn der Zustand eine handlungsrelevante Ursache hat (etwa einen nicht bereitgestellten Eintrag). `--format json` gibt stattdessen ein Dokument mit `spec_version`, `checked_at`, `exit_status` und einem `targets`-Array aus; mit `--target` enthält das Array genau einen Eintrag. Die Zustände sind `update_available`, `up_to_date`, `not_installed`, `inaccessible`, `network_error`, `invalid_artifact` und `credential_store_unavailable`.

Exit-Status: `0` sauber, `1` Konfigurations- oder Auswahlfehler, die der Updater selbst erkennt, `2` mindestens ein Ziel fehlgeschlagen. Ein Kommandozeilen-Nutzungsfehler (etwa ein ungültiger `--format`-Wert) endet mit `1`: es wurde nichts ausgeführt. Exit `2` ist Ziel-Fehlschlägen vorbehalten und wird nie für Nutzungsfehler verwendet. Die historische flache Form behält ihre bisherige Wortwahl und Exit `1`, wenn der Quellzugriff fehlschlägt.

### Installation und installierte Version

Der Updater lädt das ZIP, das zu Plattform und Architektur passt, zusammen mit der SHA-256-Datei, prüft das Archiv, sichert die Installation des Ziels, ersetzt Dateien, startet den Dienst des Ziels neu, falls es einen hat, und stellt das Backup wieder her, wenn etwas fehlschlägt. Das Archiv bestimmt seinen Inhalt: Ein `bin/`-Verzeichnis beschränkt den Austausch auf dessen Inhalt, sonst wird das Archiv-Wurzelverzeichnis installiert. Kopieren überschreibt Dateien, löscht aber nie das Installationsverzeichnis.

Die installierte Version steht in `install.json` im Installationsverzeichnis des Ziels. Ein fehlender, unlesbarer oder zu einem anderen Ziel gehörender Marker wird als `not installed` gemeldet. Der Marker wird erst nach einer erfolgreichen Installation geschrieben, damit eine fehlgeschlagene Installation nie die neue Version beansprucht. Eine Übergangsregel schützt Installationen aus der Zeit vor dem Marker: Nur für `guidance-runtime` gilt ein fehlender Marker zusammen mit `guidance-service.exe` im Installationsverzeichnis als installiert in der laufenden Version.

`guidance update install` startet einen abgekoppelten Updater und endet mit `0`, sobald dieser läuft; das Ergebnis pro Ziel und der Exit-Status des Updaters (`0` oder `2`) entstehen in diesem Prozess, nachdem die CLI beendet ist.

Die Installation ist nur unter Windows implementiert. macOS und Linux können nach Updates suchen; eine Installation dort meldet, dass sie nur unter Windows unterstützt wird. Die Release-Matrix veröffentlicht weiterhin nur Windows-Archive, bis Nicht-Windows-Artefakte veröffentlicht werden.

Das Tray-Menü zeigt den aktuell konfigurierten Update-Channel und enthält **Check for updates**.
