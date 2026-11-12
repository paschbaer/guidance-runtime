# Technical Context

> Derived from the repository (Cargo manifests, CI workflows, scripts, `config/*.example.toml`).
> Items marked **(assumption)** are inferred and not yet confirmed by the maintainers.

## Technology Stack

- **Primary language:** Rust (edition 2021), Cargo workspace with 8 member crates
- **Runtime:** Native executables; async via Tokio (`full` feature)
- **Frameworks and key crates:** `clap` (CLI), `serde`/`serde_json` (protocol, IPC, config), `toml`,
  `reqwest` (rustls-tls, release download), `semver`, `sha2` (SHA-256 asset verification),
  `zip` (archive extraction), `tempfile`, `tray-icon` + `winit` (tray), `uuid`, `tracing` +
  `tracing-subscriber` (json, env-filter)
- **Package manager:** Cargo
- **Database or storage:** None. State is the config files under `config/` plus the filesystem
  (install directory, `log_dir`)
- **Infrastructure:** Windows services (`sc.exe`), GitHub Releases as the distribution channel

## Repository Layout

```text
.
├── crates/
│   ├── protocol/    guidance-protocol   — IPC request/response types
│   ├── ipc/         guidance-ipc        — IPC client + method dispatch
│   ├── runtime/     guidance-runtime    — workload supervision core
│   ├── service/     guidance-service    — Windows service host (bin)
│   ├── cli/         guidance-cli        — `guidance` binary
│   ├── tray/        guidance-tray       — tray icon app (bin, Windows only)
│   ├── update/      guidance-update     — update check/verify/install library
│   └── updater/     guidance-updater    — self-update helper (bin)
├── config/          runtime.example.toml, updates.example.toml
├── scripts/         build.ps1, install-dev.ps1, uninstall-dev.ps1, verify-arch.ps1
├── docs/            SPECIFICATION-v1.1.md, IMPLEMENTATION_STATUS.md, SECURITY.md
├── .github/         workflows (ci.yml, release.yml), prompts, agents, skills
├── .guidance/       Guidance MCP workflow configuration
├── memory-bank/     durable project knowledge (this directory)
└── .specify/        Spec Kit artifacts
```

## Prerequisites

- **Rust:** stable toolchain, pinned via `rust-toolchain.toml` with components `rustfmt` and
  `clippy`, `profile = "minimal"`
- **PowerShell:** for `scripts/*.ps1` and the Windows service installation
- **Windows:** required for the service, tray, and update installation paths (`cfg(windows)`)

## Common Commands

```bash
# Fetch dependencies
cargo fetch

# Build (debug)
cargo build
cargo build --workspace --all-targets

# Tests
cargo test --workspace

# Lint and format (identical to CI)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Release build (PowerShell)
./scripts/build.ps1

# Reset build state
cargo clean
```

## Development Environment

- **Supported operating systems:** Windows is the implementation target. Linux and macOS are
  planned for later releases (see `docs/SPECIFICATION-v1.1.md` §1) — currently only
  `cargo build`/`test`/`clippy` run on Ubuntu in CI; service, tray, and install paths are
  `cfg(windows)`-gated.
- **Recommended editor:** Zed or VS Code (repository contains `.vscode/`)
- **Environment variables:** Document names only, never values

## Configuration

- **Local configuration files:** `config/runtime.toml` and `config/updates.toml`, created by
  copying the corresponding `*.example.toml`
  - `runtime.toml`: `log_dir`, `[[workloads]]` entries (`id`, `command`, `args`, `enabled`,
    `endpoint`, `[workloads.restart]` with `policy`, `max_attempts`, `delay_seconds`)
  - `updates.toml`: `repository`, `channel` (`stable` | `beta`)
- **Shared configuration files:** `rust-toolchain.toml`, `Cargo.toml` (workspace dependencies
  are centralized in `[workspace.dependencies]`)
- **Secret source:** none in the repository; `docs/SECURITY.md` is the reference for handling

## CI/CD

- **CI provider:** GitHub Actions
  - `.github/workflows/ci.yml`: matrix `windows-latest` and `ubuntu-latest`, native builds
  - `.github/workflows/build.yml` (`Runtime CI`): matrix `x86_64-pc-windows-msvc` and
    `aarch64-pc-windows-msvc` on `windows-latest` (x64 runner), so the arm64 leg cross-compiles
- **Validation workflow:** `cargo fmt --all -- --check` → `cargo clippy --workspace
  --all-targets -- -D warnings` → `cargo test --workspace` → `cargo build --workspace
  --release`; dependencies are cached with `Swatinem/rust-cache@v2`
- **Release workflow:** `.github/workflows/release.yml`
  - Builds both `windows-x64` and `windows-arm64` on `windows-latest` and verifies the PE machine
    type of every packaged `.exe` with `scripts/verify-arch.ps1` before uploading the artifact
  - Push to `develop` → `v1.3.0-beta.<run-number>` prerelease
  - Push to `main` → `v1.3.<run-number>` stable release, marked Latest
  - Only changes under `crates/**`, `config/**`, `scripts/**`, the Cargo files,
    `rust-toolchain.toml`, or the release workflow trigger a release
  - `[skip release]` in the head commit message suppresses the release
  - Version family controlled by `VERSION_MAJOR` / `VERSION_MINOR` in `release.yml`

## External Services and Tools

- **GitHub Releases**
  - Purpose: update distribution (ZIP asset + SHA-256 asset)
  - Configuration location: `config/updates.toml`
  - Required permissions: read access to the repository's releases
- **Windows Service Control Manager (`sc.exe`)**
  - Purpose: stop/start the `GuidanceRuntime` service during an update
  - Required permissions: local administrator (assumption)

## Constraints and Compatibility

- The updater is fail-closed: a release without a SHA-256 asset is refused.
- Update installation is Windows-only; on other platforms `install_archive` bails with an error.
- Managed workload binaries are expected to ship `guidance-service.exe`, `guidance.exe`,
  `guidance-tray.exe`, and `guidance-updater.exe` in one directory inside the archive.
- `zip = "2"` is resolved to a 2.x release; the extraction code uses `enclosed_name() -> PathBuf`.

## Troubleshooting

### An arm64 test binary cannot be executed on an x64 runner

**Symptoms:** `cargo test --target aarch64-pc-windows-msvc` compiles, then fails with
`could not execute process ... (never executed)` → `os error 216` ("This version of %1 is not
compatible with the version of Windows you're running").

**Cause:** The hosted `windows-latest` runner is x64. Windows refuses to start an ARM64 image
unless ARM64 emulation is enabled (for example on arm64 Windows itself).

**Resolution:** Keep `--no-run` on the arm64 leg so the test binaries are still linked, gate the
executing `cargo test` on the native target, and verify the produced executables with
`scripts/verify-arch.ps1` (PE machine field). Only the arm64 acceptance criterion itself needs a
real arm64 runner or device.

### `cargo clippy --workspace --all-targets -- -D warnings` fails on Linux/CI but not locally

**Symptoms:** Compile errors only on one platform, or errors in code paths that are never built
locally.

**Cause:** Platform-gated code (`#[cfg(windows)]` / `#[cfg(not(windows))]`) is not compiled on the
other OS, so a binding or type error in that arm stays invisible until the other target builds it.

**Resolution:** Always treat the CI result on both matrix OSes as authoritative; run
`cargo clippy --workspace --all-targets -- -D warnings` before pushing, not only `cargo build`.
