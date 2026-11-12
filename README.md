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
repository = "YOUR-ORG/guidance-runtime"
channel = "stable" # stable or beta
```

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
guidance update channel
guidance update channel beta
guidance update check
guidance update install --yes
```

The updater downloads the ZIP that matches the architecture it is running on (`windows-x64` or `windows-arm64`) together with the SHA-256 asset, verifies the archive, stops `GuidanceRuntime`, backs up the installation, replaces the binaries, and restarts the service. It restores the backup if replacement or restart fails. The architecture is derived automatically; set `asset_pattern` in `updates.toml` only to override it.

The tray displays the currently configured update channel and provides **Check for updates**.
