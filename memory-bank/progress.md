# Progress

## Overall Status

**Phase:** Repository Ready (partially complete)

**Summary:** The workspace builds cleanly under CI's strict clippy gate and the Guidance
configuration exists and is verified. Project-brief and product-context documentation still
depend on input from the maintainers.

## Completed

- [x] Create the Memory Bank directory structure.
- [x] Fix the release build so `cargo clippy --workspace --all-targets -- -D warnings` passes:
      `zip` 2 API adaptation, winit `ApplicationHandler` migration, CLI install binding.
- [x] Establish a green CI baseline on Windows (fmt, clippy, test).
- [x] Generate and verify the `.guidance/` configuration (template `rust`, 23 files,
      `setup_guidance_verify` reports `verified: true`).
- [x] Register the repository in the Guidance workspace registry as `guidance-runtime`.
- [x] Document the actual toolchain and commands in `techContext.md`.
- [x] Document the architecture in `systemPatterns.md`.
- [x] Fix the SOLID review findings: IPC dispatch extraction (RW-001), typed `Method` enum,
      update ports behind `Releases`/`Downloader`/`Installer`, shared update commands.
- [x] Raise workspace test coverage from 2 to 55 tests around the update pipeline and IPC mapping.
- [x] Track `Cargo.lock` (RW-002).
- [x] Draft the project constitution v1.0.0 from the resolved scaffold (five core principles plus
      technology, workflow, and governance sections).
- [x] Ratify the constitution v1.0.0 on 2026-11-12 and remove the temporary Sync Impact Report.
- [x] Implement Spec-Kit feature 001 phases 1-3: multi-target update configuration, seven-state
      outcome model, platform-aware archive suffix table, per-target fan-out with failure
      isolation, exit statuses, and CLI wiring. Workspace tests 41 → 91.
- [x] Implement feature 001 phase 4: CredentialStore port with three platform backends,
      authenticated GitHub adapter over the API asset endpoint, and credential wiring in the check
      path. Workspace tests 91 → 109.
- [x] Implement feature 001 phases 5-7 (T023-T040): structured text and JSON output with
      `--format text|json`, the `install.json` version marker as the installed-version source,
      the shared transactional per-target installer (service-less targets included, marker written
      last, full rollback), per-target install orchestration, the macOS CI leg, and the operator
      documentation in both languages plus the quickstart validation record. Workspace tests
      109 → 160 (141 in `guidance-update`).

## In Progress

None — see Change Log.

## Planned

- [ ] Define the first milestone and acceptance criteria.
- [ ] Add a service-control port so the backup/restore transaction becomes testable (RW-005).
      This gap is now explicit: the Windows service-state half of scenario 7 has no automated test.
- [ ] Record initial architectural decisions as ADRs.
- [ ] Validate the development environment end to end (install/uninstall via `scripts/*.ps1`).
- [ ] Decide the CLI-wide exit-code taxonomy: clap usage errors exit 2, which collides with the
      contract's "at least one target failed" status 2.
- [ ] Wire or remove the global `--json` flag on `guidance`; update subcommands use `--format json`.
- [ ] Add macOS/Linux release legs once non-Windows artifacts are published.

## Known Issues

- The amendment approver role is still undesignated (RW-009).
- `projectBrief.md` and `productContext.md` no longer contain template placeholders; their
  remaining open items are roadmap questions recorded in `productContext.md`.
- The Windows service-control transaction is still only partially testable: the service-state half
  of target isolation (RW-005) is not covered by an automated test because `sc.exe` is not behind a
  port. The file replacement, rollback and marker behaviour are covered.
- IPC handlers have no direct test seam until a supervisor port exists (RW-006).
- A private update target reports `no access to update source` when its credential entry is absent,
  with a provisioning hint naming the platform store target.
- The installed version comes from `install.json`; an installation from before the marker existed
  still reports the running version through a transitional rule scoped to `guidance-runtime`.
- A proxy that answers with its own authentication challenge is reported as a connectivity problem
  rather than an access problem; the transport layer does not expose the distinction.
- The credential backends are not verified against a real store by any gate; no Secret Service
  daemon or Keychain is available in CI.

## Milestones

### Milestone 1: Repository Ready

**Status:** In progress

**Exit criteria:**

- [x] Project brief is complete (owner and primary users recorded 2026-11-13).
- [x] Constitution is ratified (v1.0.0, 2026-11-12).
- [x] Build, test, lint, and run commands are documented and verified.
- [x] Architecture and important boundaries are documented.
- [x] Repository instructions reference the Memory Bank.
- [ ] No unresolved critical setup issues remain.

## Change Log

### Initial Setup

- Initialized the Memory Bank.

### 2026-11-12 — Build and Guidance setup

- Repaired the workspace build for `-D warnings` (three defects, two of them Windows-only).
- Created and verified the `.guidance/` configuration; registered the workspace.
- Replaced the template placeholders in `techContext.md`, `systemPatterns.md`,
  `activeContext.md`, `progress.md`, `remaining-work-plan.md`, and `lessonsLearned.md`.

### 2026-11-12 — SOLID review fixes

- `guidance-protocol`: typed `Method` enum replaces scattered method string literals (OCP).
- `guidance-ipc`: dispatch split into named handlers with a typed `DispatchError` (SRP).
- `guidance-update`: ports (`Releases`, `Downloader`, `Installer`) with HTTP/ZIP adapters (DIP);
  pure selection logic and checksum parsing are unit-tested; Beta channel rule is explicit.
- CLI and updater share `set_or_get_channel`/`check_update` (DRY); install paths stay separate.
- `Cargo.lock` tracked; workspace test count 2 → 55.

### 2026-11-12 — Architecture-aware updates

- Asset selection derives its suffix from the running architecture; an omitted `asset_pattern`
  no longer defaults to x64. Unknown architectures and empty overrides fail loudly.
- The configured pattern can narrow the search but never select another architecture's archive.

### 2026-11-12 — Constitution v1.0.0

- Reproduced the resolved `constitution-template` (core layer, no overrides/presets/extensions).
- Replaced the Spec Kit scaffold in `.specify/memory/constitution.md` with five core principles
  grounded in `AGENTS.md`, CI gates, the IPC contract, and `docs/SECURITY.md`, plus Technology
  Constraints, Development Workflow & Quality Gates, and Governance sections.
- Ratified 2026-11-12; the temporary Sync Impact Report was removed. The amendment approver role
  remains open (RW-009).

### 2026-11-12 — Multi-target update sources (phases 1-3)

- `guidance-update` gained a target list: a raw `ConfigFile` plus a normalized `UpdateConfig`, so
  both the historical flat form and a `[[targets]]` array parse and every validation error names
  the offending field.
- Added a closed seven-state outcome model and `UpdateRun`, with both invariants enforced at
  construction.
- `arch.rs` widened from Windows-only to a platform-and-architecture suffix table; the Windows
  values are unchanged and pinned by a regression test.
- `check_all` fans out over the configured targets and isolates each failure; exit status is 0, 1
  or 2 per the feature contract.
- A legacy flat configuration keeps its historical output wording, exit status and file shape; an
  explicit `[[targets]]` file gets one line per target.
- Three independent reviews were run through the reviewer-token handoff. Four real defects were
  fixed as a result, including an FR-003 violation where a channel change reshaped a legacy file
  and an SC-001 gap where the legacy test proved a path the CLI never took.
- Workspace tests 41 → 91.

### 2026-11-12 — Multi-target update sources (phase 4)

- Added a `CredentialStore` port with three cfg-gated backends and a redacting `Secret` newtype.
- Introduced `Access` and `AuthenticatedAsset` so a target's identity and its credential travel
  together; `browser_download_url` was removed from `Asset` rather than left unused.
- The GitHub adapter sends a bearer token only for a private target, fetches bytes from the API
  asset endpoint, and classifies 401/403/404 identically without disclosing the repository path.
- Credential wiring replaced the interim unconditional private branch; an absent entry now reports
  the entry name and the provisioning step.
- Two independent reviews were run. Both found real defects: the command backends collapsed every
  non-zero exit into "credential absent", which would have reported a locked keychain as a missing
  credential, and FR-013's provisioning step was not implemented at all.
- Workspace tests 91 → 109.

### 2026-11-12 — Multi-target update sources (phases 5-7)

- Added a dedicated `JsonDocument` (`spec_version`, RFC3339 `checked_at`, `exit_status`, `targets[]`)
  and `--format text|json` on both check entry points. The legacy flat form reuses its historical
  check, so `--format` never changes the legacy exit status. `--target` narrows the run itself.
- Added `marker.rs`: `install.json` with target, semver version and RFC3339 timestamp, an
  Absent/Unreadable/Present read state, and an atomic write. `check_one` and the legacy single-target
  path both use the marker as the selection baseline; the transitional rule is scoped to
  `guidance-runtime` and only fires for a genuinely absent marker.
- Refactored `install.rs` into one transactional body: archive discovery (`bin/` then the historical
  service directory then the root), copy-and-overwrite without deleting the install directory,
  optional service control, a full-name backup sibling, rollback that also removes files the
  rejected archive created, and the marker written last. The runtime target keeps its four-binary
  file set and `GuidanceRuntime` default even when `service_name` is omitted.
- Added `install_all` orchestration: only targets reporting an available update are installed,
  not-installed targets are skipped, failures are recorded per target, and a failing run exits 2.
- Added the macOS CI leg, corrected the Windows credential target in the docs and the provisioning
  hint (`guidance:update:<entry>`), and recorded the quickstart validation split between executed
  scenarios and those that need a live private source.
- Reviews found and fixed: a tray fail-open that reported an all-failing run as "Up to date"; a
  marker that was displayed but not used for the update decision (which could offer a downgrade);
  a backup-path collision that could delete another target's installation; additive rollback;
  a runtime-behaviour change when `service_name` was omitted; and a Windows credential hint that
  named a store target the lookup never reads.
- Final review found and fixed: `invalid_artifact` was unreachable because a newer release without
  a usable artifact was reported as up to date (the multi-target check and install now classify it
  correctly); `[[targets]]` no longer defaults a missing `install_dir` to the runtime's directory;
  install skips a not-installed target before resolving a credential, so a private target without
  an installation is not reported as an access failure.
- Workspace tests 109 → 164 (147 in `guidance-update`).

### 2026-11-13 — Cross-platform test fixtures (Linux CI leg)

- The first `ubuntu-latest` run failed 23 `guidance-update` tests. Two root causes: a credential
  assertion that rejected the legitimate Linux argument vector, and fixtures that hard-coded
  `windows-*.zip` while selection compared against the running platform's suffix, so every check
  collapsed to `invalid_artifact` (21 failures downstream of that one fixture constant).
- Fixtures now derive the archive from `asset_suffix_for_arch(running_arch())`; a
  `other_arch_zip()` helper supplies a provably foreign archive. Architecture-crossing tests keep
  explicit suffixes, and the omitted-pattern test pins an explicit per-host table instead of
  mirroring the implementation it verifies.
- The credential-command test asserts that exactly one argument mentions the entry — the entry
  itself — which is the property that fails when an entry is joined into a shell string.
- Verified by running the test suite on both hosts rather than reasoning about it: Windows (fmt,
  clippy `-D warnings`, 164 tests) and the full workspace under WSL/Linux (159 tests). The Linux run
  also caught a second, Windows-invisible defect: a checksum fixture whose text still named the
  literal `windows-x64.zip`.
- CI confirms it on all three legs (run 5, commit `28737dd`): `windows-latest`, `ubuntu-latest` and
  `macos-latest` completed `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace` and
  the release build successfully. Linux resolves `linux-x86_64.zip`, so the stale RW-008 finding
  was closed as handled.

### 2026-11-13 — Ownership recorded

- `projectBrief.md`: owner and primary users recorded as "User" (previously unconfirmed); the CI
  success criterion names all three matrix legs. `productContext.md` answers the owner/user
  question.
- Still open by design: which role approves constitution amendments (RW-009), and whether
  Linux/macOS support is a committed roadmap item.

### 2026-11-13 — Concrete update-source repositories

- Replaced the `YOUR-ORG/…` placeholders with the real repositories: `paschbaer/guidance-runtime`
  (public source repo) and `paschbaer/guidance-dist` (release-only, private, target `guidance`).
  Applied to `config/updates.example.toml`, both READMEs, and the spec's contract and data-model
  examples.
- Parser-test fixtures keep the neutral `ORG/guidance-dist` sample value; they assert parsing, not
  which repository an operator configures.
- Mirroring into the private repository remains the `guidance` product's pipeline step; this
  repository's `release.yml` publishes only its own releases, as `research.md` (R8) already states.
