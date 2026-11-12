# Implementation Plan: Multi-Target Update Sources

**Branch**: `001-multi-target-update-sources` | **Date**: 2026-11-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-multi-target-update-sources/spec.md`

**Research**: [research.md](./research.md) · **Data model**: [data-model.md](./data-model.md) ·
**Contract**: [contracts/update-cli.md](./contracts/update-cli.md) · **Validation**:
[quickstart.md](./quickstart.md)

## Summary

Turn the single-source updater into a fan-out updater over a configurable list of targets. Each
target declares its own release source, visibility, credential entry, channel and install
directory. Private targets are read from a separate private artifact repository through the
authenticated GitHub API, with the token resolved from the platform's native per-user credential
store. Installed versions come from a per-target version marker; a target without a marker is
reported as "not installed". Every target reports one of seven closed outcome states, in
human-readable and opt-in JSON form, and a failure in one target never prevents the others from
being checked.

Two findings shape the work and are not cosmetic:

1. **The private source is currently unreachable by construction.** `github.rs` sends no auth
   header and `select` builds download URLs from `browser_download_url`, which returns `404` for a
   private repository. Both the list and the artifact fetch must move to the authenticated API.
2. **Non-Windows platforms cannot currently check for updates at all.** `arch.rs` maps
   `x86_64`/`aarch64` to Windows-only suffixes and `KNOWN_ARCH_SUFFIXES` contains only Windows
   archives, so `running_arch()` fails before any network call on Linux or macOS. Since the spec
   puts macOS and Linux in scope, this table must become platform-aware.

## Technical Context

**Language/Version**: Rust, edition 2021, workspace pinned by `rust-toolchain.toml`

**Primary Dependencies**: `reqwest` (rustls-tls, json), `serde`/`serde_json`, `toml`, `semver`,
`sha2`, `zip`, `tempfile`, `clap`. New: a platform credential backend (`windows-sys` for
Credential Manager; process invocations for macOS Keychain and Linux Secret Service), `serde_json`
promoted from transitive to a direct `workspace.dependencies` entry.

**Storage**: Configuration `updates.toml` (operator-owned); per-target `install.json` version
marker inside each install directory; credentials never persisted by this crate

**Testing**: `cargo test --workspace` with the existing port-based doubles
(`FixedReleases`/`FixedDownloader`); new doubles for the credential store and for authenticated
asset fetches. No test performs real network I/O.

**Target Platform**: Windows (service + transactional install), Linux and macOS (update check and
install without service control). Windows-only code stays `cfg(windows)`-gated with a compiling
non-Windows arm.

**Project Type**: Desktop application with a Windows service, a CLI, a tray binary and a
standalone updater — an eight-crate Cargo workspace

**Performance Goals**: Not a stated requirement. A check run is operator-triggered and bounded by
the number of configured targets; sequential fan-out is sufficient (R4).

**Constraints**: Secrets must never appear in configuration, logs, output or error text. Checksum
verification is mandatory and fail-closed. Each target's install transaction must be able to
restore the previous state. The Windows runtime target's existing behaviour must not change.

**Scale/Scope**: A handful of targets (2 in practice), one machine, one operator. The affected
surface is `crates/update` (all modules), `crates/updater`, `crates/cli`'s update subcommands,
`config/updates.example.toml` and the docs.

## Constitution Check

*GATE: evaluated before Phase 0, re-evaluated after Phase 1.*

| Principle | Gate | Status |
|---|---|---|
| **I. Fail-Closed Safety** | Update refused without checksum; failed transaction restores backup; partial application never reported as success | **PASS** — FR-018/FR-019 preserved, FR-020a/b extend the guarantee to the service-less target, marker written last (R3, R5) |
| **II. Test Evidence Before Claims** | Behaviour change carries tests; fmt/clippy/test reported as run; both CI legs authoritative for `cfg` code | **PASS with a required action** — every new outcome state and the auth path carry port-level tests. The macOS gap is documented rather than claimed (R6, quickstart scenario 9) |
| **III. Stable Protocol Surface** | IPC wire names pinned; no raw method strings | **N/A for IPC** — this feature adds no IPC method. The CLI output schema is treated as an external contract and versioned (`spec_version`) instead |
| **IV. Minimal, Review-Gated Change** | No unrequested changes; dependencies inverted through ports; SOLID boundaries | **PASS** — `CredentialStore` is a new port; `Releases`/`Downloader`/`Installer` keep their direction. **Deviation below** |
| **V. Platform Boundaries Are Explicit** | `cfg`-gated platform code with a compiling fallback; new architectures need a `KNOWN_ARCH_SUFFIXES` entry and a matrix leg; hardcoded default is a defect | **FAIL as-is, corrected by this plan** — `KNOWN_ARCH_SUFFIXES` is Windows-only while the spec requires three platforms. R6 widens it; the release matrix leg is deferred with a stated reason |

### Principle IV deviation (requires justification)

`Installer::install(&self, config: &UpdateConfig, bytes: &[u8])` currently takes the whole
configuration. A multi-target installer needs the target, not the config. Changing this trait is a
deviation from "minimal change" only in appearance: leaving it would force the installer to
re-derive which target it is installing, which is the ambiguity that produces cross-target
overwrites (FR-020). The signature changes to accept the resolved target plus the archive bytes,
and the existing runtime call site is adapted in the same change.

**Operator decision (2026-11-12)**: the signature change is approved. Keep the existing trait and
adapt its signature rather than introducing an additional parallel trait.

### Principle V deviation (justified, with a follow-up trigger)

The spec requires macOS and Linux as update clients; the release workflow publishes only Windows
artifacts. Add a macOS leg to `ci.yml` so the platform claim is testable, but do **not** add
release matrix legs for artifacts the private product does not yet publish. The trigger to add
them is the private product publishing non-Windows archives; until then the suffix table covers
the platforms and a missing artifact is reported as `invalid_artifact`, not as a crash.

**Operator decision (2026-11-12)**: the macOS CI leg is in scope and is tracked as its own task
(it touches `.github/workflows/ci.yml`, which is infrastructure rather than application code).
Release matrix legs remain out of scope until the trigger above fires.

## Project Structure

### Documentation (this feature)

```text
specs/001-multi-target-update-sources/
├── plan.md              # This file
├── research.md          # Phase 0 — decisions R1–R8
├── data-model.md        # Phase 1 — entities, states, invariants
├── quickstart.md        # Phase 1 — validation scenarios
├── contracts/
│   └── update-cli.md    # Phase 1 — CLI, exit codes, JSON schema, config schema
├── checklists/
│   └── requirements.md  # Specify output
├── clarify-report.md    # Clarify output
└── tasks.md             # Phase 2 (/speckit.tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/update/src/
├── lib.rs               # Config model: legacy + [[targets]], UpdateTarget, composition root
├── ports.rs             # Releases, Downloader, Installer, + CredentialStore
├── github.rs            # Authenticated adapter: list + asset endpoint + checksum
├── credentials.rs       # NEW — CredentialStore port impls per platform (cfg-gated)
├── arch.rs              # Platform-aware suffix table (widened from Windows-only)
├── selection.rs         # Per-target selection; asset ref carries an authenticated reference
├── install.rs           # Shared TransactionalInstall; optional service name
├── marker.rs            # NEW — version marker read/write, transitional legacy rule
├── report.rs            # NEW — TargetState/TargetCheck/UpdateRun, text + JSON rendering
├── command.rs           # Fan-out orchestration, per-target error isolation, exit status
└── tests.rs             # Existing doubles extended; new credential/store doubles

crates/updater/src/main.rs   # CLI surface for the standalone updater
crates/cli/src/main.rs       # `guidance update check|install|channel --target …`
config/updates.example.toml  # Documented legacy + multi-target forms
docs/IMPLEMENTATION_STATUS.md# Update chapter
```

**Structure Decision**: Keep the single-crate module layout of `crates/update`; no new crate. The
existing port/adapters split already matches the constitution's inversion requirement, and the
feature is a widening of that pipeline rather than a new subsystem. Three new modules are added
where the plan needs a seam that tests can drive: `credentials` (platform boundary), `marker`
(installed-state source) and `report` (outcome model). `command.rs` becomes the fan-out
orchestrator and keeps rendering and orchestration separate.

## Design decisions carried from research

| # | Decision | Rationale anchor |
|---|---|---|
| D1 | Release list and artifact bytes fetched through the authenticated API; `browser_download_url` never used for private sources | R1, FR-017 |
| D2 | `Asset` gains `id`; archive and checksum resolve through the same authenticated reference | R1 |
| D3 | `CredentialStore` port with Windows/macOS/Linux backends; absent vs. unusable store are distinct outcomes | R2, FR-011a |
| D4 | Version marker `install.json` per target, written last, mismatch → not installed | R3, FR-009a–c |
| D5 | Sequential fan-out with per-target `Result` isolation; exit `0`/`1`/`2` | R4, FR-008 |
| D6 | Transactional install factored out, service name optional | R5, FR-020a/b |
| D7 | `arch.rs` widened to `(platform, arch) → suffix`, Windows mapping byte-identical | R6 |
| D8 | Closed `TargetState` enum in a versioned JSON document | R7, SC-010 |
| D9 | Mirroring contract recorded, implemented in the private product's pipeline | R8, FR-027–029 |

## Phases

- **Phase 0** — `research.md` complete; all spec open questions resolved. Done.
- **Phase 1** — `data-model.md`, `contracts/update-cli.md`, `quickstart.md` complete. Done.
- **Phase 2** — `tasks.md` via `/speckit.tasks`, ordered so the pipeline can be validated after
  each slice (config model → platform suffixes → auth adapter → marker → fan-out report →
  transactional install → macOS CI leg → docs).

### Confirmed scope additions (2026-11-12)

| Item | Decision | Where it lands |
|---|---|---|
| macOS CI leg | In scope; own task | `.github/workflows/ci.yml` — new `macos-latest` matrix entry |
| `Installer` signature change | In scope; adapt the existing trait | `crates/update/src/ports.rs`, `install.rs` and the runtime call site |

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| `Installer` trait signature change | The installer must know which target it installs; deriving it from the config reintroduces the cross-target overwrite risk FR-020 forbids | Keeping the config-wide signature would make the target implicit and untestable per target |
| New `CredentialStore` port plus three backends | The spec requires macOS and Linux alongside Windows, and the constitution forbids a Windows-only credential default | A single Windows-only store would contradict the operator's explicit platform requirement; a third-party keyring crate hides the platform boundary Principle V requires to stay explicit |
| Three new modules in `crates/update` | Each seam must be drivable by tests without network or OS access | Inlining credential lookup, marker I/O and outcome rendering into `command.rs` would make the fan-out untestable and mix rendering with policy |

## Post-Design Constitution Re-Check

| Principle | After Phase 1 | Note |
|---|---|---|
| I | PASS | Every failure state leaves the previous installation and marker intact; checksum stays mandatory |
| II | PASS with open action | Test plan covers all seven states and the auth path; macOS execution remains an explicit "not executed" until CI covers it |
| III | PASS | No IPC change; CLI schema versioned via `spec_version` |
| IV | PASS (justified deviation recorded) | Installer signature change justified above |
| V | PASS with follow-up trigger | Platform suffixes widened; release matrix legs deferred with a named trigger |

### Open constitution gap (not resolved by this feature)

`constitution.md` § *Configuration and secrets* states that secrets MUST NOT appear in the
repository, TOML files or documentation, and that **environment variables are documented by name
only**. The chosen mechanism — a named entry in the platform's protected per-user credential
store — is strictly stricter than an environment variable, but the constitution does not enumerate
it.

This is **not** treated as a violation: the requirement is satisfied with a safer mechanism. It is
recorded here because the constitution is the governing authority and a future reader should not
have to rediscover that its secret-handling clause is silent on OS credential stores. Resolving it
belongs in a separate constitution amendment, since the constitution itself forbids amendments that
carry unrelated implementation work.

Suggested amendment wording for that separate change: *"Secrets MAY alternatively live in the
platform's protected per-user credential store, referenced by name from configuration."*
