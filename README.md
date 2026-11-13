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
| Windows | Credential Manager, generic credential, target `guidance:update:<entry>` |
| macOS | `security add-generic-password -s guidance-update -a <entry> -w` |
| Linux | `secret-tool store --label='Guidance' service guidance-update username <entry>` |

Grant the token **read access to the release artifacts of that repository only**. It must not grant source-repository access and must not grant write access. When the entry is missing, the report says `no access to update source` and names the entry to provision.

### Output and exit status

A check prints one line per target, plus an indented detail line where the state has an actionable cause (for example an unprovisioned entry). `--format json` emits one document instead, with `spec_version`, `checked_at`, `exit_status` and a `targets` array; with `--target` that array holds one entry. The outcome states are `update_available`, `up_to_date`, `not_installed`, `inaccessible`, `network_error`, `invalid_artifact` and `credential_store_unavailable`.

Exit status: `0` clean, `1` configuration or selection errors detected by the updater, `2` at least one target failed. A command-line usage error (for example an invalid `--format` value) also exits `2`, because clap uses that status. The historical flat form keeps its pre-feature wording and its exit `1` when the source read fails.

### Installation and installed version

The updater downloads the ZIP that matches the platform and architecture it is running on together with the SHA-256 asset, verifies the archive, backs up the target's installation, replaces files, restarts the target's service when it has one, and restores the backup if anything fails. The archive determines its content: a `bin/` directory limits replacement to its contents; otherwise the archive root is installed. Copying overwrites files but never deletes the installation directory.

The installed version comes from `install.json` in the target's installation directory. A missing, unreadable or target-mismatched marker is reported as `not installed`. The marker is written only after a successful installation, so a failed install cannot claim the new version. A transitional rule protects installations from before the marker existed: for `guidance-runtime` only, no marker plus `guidance-service.exe` in the installation directory is read as installed at the running version.

`guidance update install` starts a detached updater and exits `0` once it is running; the per-target result and the updater's own exit status (`0` or `2`) are produced by that process after the CLI exits.

Installation is Windows-only. macOS and Linux can check for updates; an install there reports that installation is supported on Windows only. The release matrix still publishes Windows archives only, until non-Windows artifacts are published.

The tray displays the currently configured update channel and provides **Check for updates**.
