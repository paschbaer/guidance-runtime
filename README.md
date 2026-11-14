# Guidance Runtime Service

Windows service, CLI, tray application, MCP workload supervisor, and GitHub Release updater written in Rust. For German documentation, see [README.de.md](README.de.md).

## Build

```powershell
.\scripts\build.ps1
```

Release binaries are written to `target\release`: `guidance-service.exe`, `guidance.exe`, `guidance-tray.exe`, and `guidance-updater.exe`.

## Configure and install

```powershell
Copy-Item .\config\runtime.example.toml .\config\runtime.toml
Copy-Item .\config\updates.example.toml .\config\updates.toml
.\scripts\install-dev.ps1
```

Set the GitHub repository and channel in `updates.toml`:

```toml
repository = "paschbaer/guidance-runtime"
channel = "stable" # stable or beta
```

This flat form remains a single target named `guidance-runtime` and keeps its historical output. It stays flat when a channel is changed. To maintain more than one product, use the `[[targets]]` form described under [Updates](#updates).

## Initial installation and configuration

This section describes a fresh (green-field) installation on an empty Windows system. Upgrading an existing installation is covered under [Updates](#updates). A Rust toolchain is **not** required on target systems — it is only needed when installing from source.

### Prerequisites

- Windows with an administrator PowerShell (service and update installation are Windows-only).
- Network access to GitHub Releases for the configured update sources.
- Only for installing from source: a Rust toolchain; `rust-toolchain.toml` pins the version and components, and `cargo` picks them up automatically.

### 1. Obtain the binaries

- **From a release (target systems):** download `guidance-runtime-<version>-<arch>.zip` together with its SHA-256 file from the repository's GitHub Releases page and verify the checksum. The archive contains `bin/` with the four executables and `config/` with the example TOML files.
- **From source (development):** run `scripts\build.ps1`; the binaries are written to `target\release`.

### 2. Install the runtime

Copy `guidance-service.exe`, `guidance.exe`, `guidance-tray.exe` and `guidance-updater.exe` to `C:\Program Files\Guidance Runtime`, create `C:\ProgramData\Guidance Runtime\config` and `C:\ProgramData\Guidance Runtime\logs`, and place `runtime.toml` and `updates.toml` there, starting from the `*.example.toml` files. A later self-update replaces exactly these four binaries in the installation directory, so this layout matches what the updater maintains.

- **From a repository checkout:** run `scripts\install-dev.ps1` (as administrator) — it performs all of this and registers the service; `scripts\uninstall-dev.ps1` removes the installation again.
- **From a release archive:** register and start the service manually:

Plain quoting does not survive `sc.exe`: it parses its own command line and consumes the quotes, and the escaping that would survive differs between `cmd.exe`, Windows PowerShell and PowerShell 7. `New-Service` avoids that entirely — it passes the binary path straight to the service manager, so the quoted command line lands verbatim in the service's `ImagePath`:

```powershell
New-Service -Name GuidanceRuntime -BinaryPathName '"C:\Program Files\Guidance Runtime\guidance-service.exe" --config "C:\ProgramData\Guidance Runtime\config\runtime.toml"' -StartupType Automatic -DisplayName 'Guidance Runtime Service'
sc.exe failure GuidanceRuntime reset= 86400 actions= restart/5000/restart/15000/restart/60000
sc.exe start GuidanceRuntime
```

In `cmd.exe`, the equivalent is `sc.exe create GuidanceRuntime binPath= "\"C:\Program Files\Guidance Runtime\guidance-service.exe\" --config \"C:\ProgramData\Guidance Runtime\config\runtime.toml\"" start= auto DisplayName= "Guidance Runtime Service". Verify with `sc.exe qc GuidanceRuntime`: `BINARY_PATH_NAME` must contain the quotes.

Workloads and the log directory are maintained in `C:\ProgramData\Guidance Runtime\config\runtime.toml` (`log_dir`, `[[workloads]]`); restart the service after changing it. Workload output lands in `<log_dir>\<workload-id>.log`; the service's own events (lifecycle, spawn failures, restart decisions) go to `C:\ProgramData\Guidance Runtime\logs\service.log.<date>` (daily rotation, JSON), independent of `log_dir`.

### 3. Configure the self-updater

The binaries read the update configuration from `C:\ProgramData\Guidance Runtime\config\updates.toml` by default — everything must be entered there, not only in the repository copy.

- **Self-update of `guidance-runtime`:** the flat form with `repository` and `channel` (`stable` or `beta`) as shown under [Configure and install](#configure-and-install).
- **Make `guidance` installable:** this requires the `[[targets]]` form (delete the flat keys; never mix the two forms). See [Multiple update targets](#multiple-update-targets) for the example: a target `guidance` with `repository = "paschbaer/guidance-dist"`, `visibility = "private"`, `credential_entry`, `channel`, and `install_dir` (required, e.g. `C:/Program Files/Guidance`).
- **Provision the private target:** once per user, create a generic credential in Windows Credential Manager with the target name `guidance:update:guidance-update`; the token must grant read access to that repository's release artifacts only. See [Private sources and credential provisioning](#private-sources-and-credential-provisioning).
- **Make `guidance` runnable:** a target without `service_name` is installed without service control; add its installation directory (e.g. `C:\Program Files\Guidance`) to `PATH` so `guidance` can be called from anywhere.

### 4. Install `guidance` for the first time and verify

The updater updates existing installations: a target without an `install.json` marker in its `install_dir` is reported `not_installed` and skipped by `guidance update install`. The **first installation of `guidance` is therefore manual**, from its release archive on `paschbaer/guidance-dist`:

```powershell
# Download the ZIP and SHA-256 from https://github.com/paschbaer/guidance-dist/releases
# (GitHub sign-in required), verify the checksum, then:
Expand-Archive .\guidance-dist-1.2.0-windows-x64.zip -DestinationPath .
Copy-Item .\guidance-dist-1.2.0-windows-x64\* "C:\Program Files\Guidance\" -Recurse -Force
```

(Archive and asset names are examples — the real names are shown on the release. If the archive root contains a `bin/` directory, copy its contents into `C:\Program Files\Guidance`.)

Then write the version marker the updater relies on — `C:\Program Files\Guidance\install.json`:

```json
{
  "target": "guidance",
  "version": "1.2.0",
  "installed_at": "2026-11-13T10:00:00Z"
}
```

`target` must match the target name from `updates.toml`, `version` the installed release (semver). From then on the updater manages the product:

```powershell
& "C:\Program Files\Guidance Runtime\guidance.exe" update check   # reports guidance as up to date
& "C:\Program Files\Guidance Runtime\guidance.exe" update install --target guidance --yes
```

On a development installation, `target\release\guidance.exe` works the same way.

## Git flow and automatic releases

```text
feature/* -> develop -> beta release
                     -> pull request -> main -> stable release
```

- Pushes to `develop` create `v1.3.0-beta.<run-number>` GitHub prereleases.
- Pushes to `main` create `v1.3.<run-number>` stable GitHub releases and mark them as Latest.
- Only changes under `crates/**`, `config/**`, `scripts/**`, the Cargo files, `rust-toolchain.toml`, or the release workflow trigger a release.
- Add `[skip release]` to the head commit message to suppress a release.
- Release tags are checked twice, before the build and immediately before publication. Existing tags or releases are never overwritten.

Recommended repository rules:

1. Protect `main` and require pull requests from `develop`.
2. Require the CI workflow and at least one approval.
3. Disable force pushes and tag deletion.
4. Protect the `stable` GitHub Environment with required reviewers. The `beta` environment can publish automatically.

The version family is controlled by `VERSION_MAJOR` and `VERSION_MINOR` in `.github/workflows/release.yml`. Change these values intentionally when starting a new release line.

## Updates

```powershell
guidance update channel [--target NAME]
guidance update channel beta [--target NAME]
guidance update check [--format text|json] [--target NAME]
guidance update install [--target NAME] --yes
```

`--target` narrows a check or install to one configured target. Without it, a check covers every target and an install applies every target that has an available update. `--yes` is mandatory for any installation.

### Channels

Two channels exist, selected per target with the `channel` key. A missing key means `stable`; values are case-insensitive and anything other than `stable` or `beta` is rejected (`channel must be stable or beta`).

- **`stable`** (default) — offers published releases only: no GitHub prerelease flag and no prerelease suffix in the tag (for example `v1.3.7`). These come from `main` pushes.
- **`beta`** — additionally offers prereleases whose prerelease identifier starts with `beta.`, the shape the release workflow publishes from `develop` (for example `v1.3.0-beta.42`). Other prerelease labels such as `alpha.` or `rc.` are never offered, and drafts are never offered on either channel.

Only versions above the installed one are ever offered. `guidance update channel beta [--target NAME]` rewrites the configuration in place; a flat file stays flat and a `[[targets]]` file stays an array.

### Multiple update targets

A single product can keep the flat form above. To maintain several products with one updater, declare them as `[[targets]]` (do not mix the two forms):

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

Two targets must not share an `install_dir`, and no target's `install_dir` may equal another target's rollback directory. `install_dir` is required in this form; only the flat form keeps its historical default. A target named `guidance-runtime` keeps the service `GuidanceRuntime` even if `service_name` is omitted; any other target without `service_name` is installed without service control.

### Private sources and credential provisioning

A private target names a credential-store entry in `credential_entry`. The secret itself is never stored in the configuration. Provision the entry once per user:

| Platform | Command |
|---|---|
| Windows | Credential Manager, generic credential, target `guidance:update:<entry>` — details below |
| macOS | `security add-generic-password -s guidance-update -a <entry> -w` |
| Linux | `secret-tool store --label='Guidance' service guidance-update username <entry>` |

Grant the token **read access to the release artifacts of that repository only**. It must not grant source-repository access and must not grant write access. When the entry is missing, the report says `no access to update source` and names the entry to provision.

#### Windows step by step, using `guidance` (`paschbaer/guidance-dist`) as the example

The starting point is this entry in `C:\ProgramData\Guidance Runtime\config\updates.toml`:

```toml
[[targets]]
name = "guidance"
repository = "paschbaer/guidance-dist"
visibility = "private"
credential_entry = "guidance-update"
channel = "stable"
install_dir = "C:/Program Files/Guidance"
```

This determines the credential's target name: the fixed prefix `guidance:update:` plus the value of `credential_entry` → **`guidance:update:guidance-update`**. That exact string is entered below, field by field. Version numbers in the example outputs are examples.

**Prerequisite — access to the repository (the intended model).** Every user works with their own GitHub account: the owner explicitly invites the accounts to the repository (in the repository: **Settings → Collaborators → Add people**, role `read`) or, for organization-owned repositories, through a team with read access. Because `guidance-dist` is a pure dist repository (release artifacts only, no source code), `read` is fully sufficient — it grants exactly access to the releases and nothing more. The token can only exercise rights its GitHub account already has; a token from an account without repository access later produces the same `no access to update source` message as a missing entry — the browser check distinguishes the two cases (404 = no access). Every user creates and provisions their **own** token; do not use shared tokens — they cannot be revoked individually, expire for everyone at once, and are not attributable.

**Step 0 — see the state without the entry.** `guidance update check` reports (rendering as in the code):

```text
guidance-runtime  up to date (1.3.7, stable)
guidance  no access to update source
  no credential named 'guidance-update' is provisioned on this machine; provision it as the generic Credential Manager credential 'guidance:update:guidance-update'
```

The indented line already names the target to create.

**Step 1 — create the token at GitHub.** github.com → profile icon → **Settings** → **Developer settings** → **Personal access tokens** → **Fine-grained tokens** → **Generate new token**:

- *Token name:* `guidance-updates`; *Expiration:* per your own policy (e.g. 90 days).
- *Resource owner:* `paschbaer`.
- *Repository access:* **Only select repositories** → `paschbaer/guidance-dist`.
- *Permissions → Repository permissions:* **Contents: Read-only** (releases and their assets fall under it; `Metadata` is set to Read-only automatically). Everything else gets no access.
- **Generate token** → copy the token (`github_pat_…`) immediately; it is shown only once.

**Step 2a — create the entry via the GUI.** Control Panel → User Accounts → **Credential Manager** → the **Windows Credentials** tab → **Add a generic credential**:

| Field | Value |
|---|---|
| Internet or network address | `guidance:update:guidance-update` |
| User name | `paschbaer` (anything works — the updater reads only the password field) |
| Password | paste the `github_pat_…` token |

Save with **Save**. Field labels may differ slightly between Windows language versions.

**Step 2b — or via `cmdkey`:**

```powershell
cmdkey /generic:guidance:update:guidance-update /user:paschbaer /pass
```

`/pass` without a value prompts for the token without echoing it — it never lands in shell history or the process list. The form `/pass:<token>` leaves the token in plain text in the history and should be avoided. Alternatively, the WiX MSI installer offers an exit-dialog checkbox (default off) that starts exactly this `cmdkey` prompt in the user's own session; the MSI itself carries no token.

**Step 3 — verify the entry:**

```powershell
cmdkey /list | findstr guidance:update
```

Expected (labels depend on the Windows language; the target line is what matters):

```text
    Target: LegacyGeneric:target=guidance:update:guidance-update
    Type: Generic
```

**Step 4 — check again.** `guidance update check` now reports:

```text
guidance  not installed
```

That means access works, but `guidance` is not installed yet (the `install.json` marker is missing) — the first installation is manual, see step 4 of the installation section. Afterwards the check reports `guidance  up to date (1.2.0, stable)` or, for a newer release, `guidance  update available: 1.2.0 -> 1.3.0 (stable)`, and `guidance update install --target guidance --yes` handles all following updates.

Notes: the entry is **per Windows account** — the account that creates it must also be the account that runs `guidance update`. The type must be **generic** (not a domain/web credential). An empty password value counts as "not provisioned". If the check still reports `no access to update source` despite the entry, the token lacks read access to the releases (issue a new token); `credential_store_unavailable` means the store itself is not readable.

### Output and exit status

A check prints one line per target, plus an indented detail line where the state has an actionable cause (for example an unprovisioned entry). `--format json` emits one document instead, with `spec_version`, `checked_at`, `exit_status` and a `targets` array; with `--target` that array holds one entry. The outcome states are `update_available`, `up_to_date`, `not_installed`, `inaccessible`, `network_error`, `invalid_artifact` and `credential_store_unavailable`.

Exit status: `0` clean, `1` configuration or selection errors detected by the updater, `2` at least one target failed. A command-line usage error (for example an invalid `--format` value) exits `1`: nothing was run. Exit `2` is reserved for target failures and is never used for usage errors. The historical flat form keeps its pre-feature wording and its exit `1` when the source read fails.

### Installation and installed version

The updater downloads the ZIP that matches the platform and architecture it is running on together with the SHA-256 asset, verifies the archive, backs up the target's installation, replaces files, restarts the target's service when it has one, and restores the backup if anything fails. The archive determines its content: a `bin/` directory limits replacement to its contents; otherwise the archive root is installed. Copying overwrites files but never deletes the installation directory.

The installed version comes from `install.json` in the target's installation directory. A missing, unreadable or target-mismatched marker is reported as `not installed`. The marker is written only after a successful installation, so a failed install cannot claim the new version. A transitional rule protects installations from before the marker existed: for `guidance-runtime` only, no marker plus `guidance-service.exe` in the installation directory is read as installed at the running version.

`guidance update install` starts a detached updater and exits `0` once it is running; the per-target result and the updater's own exit status (`0` or `2`) are produced by that process after the CLI exits.

Installation is Windows-only. macOS and Linux can check for updates; an install there reports that installation is supported on Windows only. The release matrix still publishes Windows archives only, until non-Windows artifacts are published.

The tray displays the currently configured update channel and provides **Check for updates**.
