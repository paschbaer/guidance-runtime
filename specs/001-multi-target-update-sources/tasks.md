---

description: "Task list for Multi-Target Update Sources"
---

# Tasks: Multi-Target Update Sources

**Input**: Design documents from `specs/001-multi-target-update-sources/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/update-cli.md](./contracts/update-cli.md)

**Tests**: Included. Constitution Principle II ("Test Evidence Before Claims", NON-NEGOTIABLE)
requires every behaviour change to carry a test or an explicit statement of why no seam exists.
Test tasks are therefore first-class, not optional.

**Organization**: Grouped by user story so each story is independently implementable and testable.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Parallelizable — different files, no dependency on an incomplete task
- **[Story]**: US1 / US2 / US3 / US4 per [spec.md](./spec.md)
- Exact file paths appear in every task

## Path Conventions

Single Cargo workspace at the repository root. All paths are workspace-relative:

- `crates/update/src/` — the update pipeline (most work)
- `crates/updater/src/main.rs` — standalone updater CLI
- `crates/cli/src/main.rs` — `guidance update …` subcommands
- `crates/runtime/src/lib.rs` — workload model, unaffected
- `config/`, `docs/`, `.github/workflows/` — non-code surfaces

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Make the shared dependency surface ready. Nothing here changes behaviour.

- [ ] T001 Add `serde_json` to `[workspace.dependencies]` and reference it from `crates/update/Cargo.toml`; add `windows-sys` behind a `cfg(windows)` target dependency for Credential Manager
- [ ] T002 [P] **Before T001**, record the pre-change baseline: run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and store the results in the task notes. Capturing this after the manifest change makes the comparison meaningless (Principle II)
- [ ] T003 [P] Document the release-side contract the updater consumes in `docs/IMPLEMENTATION_STATUS.md` (archive naming, checksum asset presence, mandatory mirroring) without claiming it is implemented

**taskmeta** (T001)

```yaml
id: T001
owner_files: [Cargo.toml, crates/update/Cargo.toml]
crates: [update]
depends_on: []
exclusive: [workspace-manifest, cargo-lock]
estimated_minutes: 25
```

**taskmeta** (T002)

```yaml
id: T002
owner_files: []
crates: []
depends_on: []
exclusive: []
sequencing_note: must run before T001 so the baseline predates the manifest change
estimated_minutes: 20
```

**taskmeta** (T003)

```yaml
id: T003
owner_files: [docs/IMPLEMENTATION_STATUS.md]
crates: []
depends_on: []
exclusive: [docs/implementation-status]
estimated_minutes: 15
```

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The configuration model, outcome model and platform suffix table that every story
reads. **No user story can start before this phase completes.**

- [ ] T004 Create `crates/update/src/report.rs` with the closed `TargetState` enum — exactly `update_available`, `up_to_date`, `not_installed`, `inaccessible`, `network_error`, `invalid_artifact`, `credential_store_unavailable` — plus `TargetCheck` and `UpdateRun` per [data-model.md](./data-model.md)
- [ ] T005 [P] Add invariant tests in `crates/update/src/tests.rs`: `installed_version.is_none()` ⟺ `state == not_installed`; `offered_version.is_some()` ⟹ `state == update_available`; a run of `n` targets yields exactly `n` checks
- [ ] T006 Rework `UpdateConfig` in `crates/update/src/lib.rs` to support `[[targets]]` with fields `name`, `repository`, `visibility` (default `public`), `credential_entry`, `channel`, `install_dir`, `service_name`, `asset_pattern`, while the legacy flat keys still parse into a single target named `guidance-runtime`
- [ ] T007 Add load-time validation in `crates/update/src/lib.rs`: reject empty `targets`, duplicate `name`, empty `name`, `visibility = "private"` without `credential_entry`, duplicate `install_dir`, `asset_pattern` not ending in `.zip` or blank, and legacy keys combined with `targets` — each error must name the offending field
- [ ] T008 [P] Add config tests in `crates/update/src/tests.rs`: legacy file yields one `guidance-runtime` target with unchanged behaviour; each validation rule produces its named error; loading does not mutate the file; a configuration carrying `credential_entry` accepts no platform-specific store key and exposes an identical parsed schema on Windows, macOS and Linux (FR-011b)
- [ ] T009 Widen `crates/update/src/arch.rs` from Windows-only suffixes to a `(platform, arch) → suffix` mapping covering `windows-x64.zip`, `windows-arm64.zip`, `macos-x86_64.zip`, `macos-aarch64.zip`, `linux-x86_64.zip`, `linux-aarch64.zip`; extend `KNOWN_ARCH_SUFFIXES` accordingly while keeping every existing Windows suffix byte-identical
- [ ] T010 [P] Add `arch.rs` tests: the Windows mapping stays byte-identical to today (regression guard), each new platform/arch pair resolves, an unknown platform or arch is rejected by name

**taskmeta** (T004)

```yaml
id: T004
owner_files: [crates/update/src/report.rs, crates/update/src/lib.rs]
crates: [update]
depends_on: [T001]
exclusive: []
estimated_minutes: 40
```

**taskmeta** (T005)

```yaml
id: T005
owner_files: [crates/update/src/tests.rs]
crates: [update]
depends_on: [T004]
exclusive: []
estimated_minutes: 25
```

**taskmeta** (T006)

```yaml
id: T006
owner_files: [crates/update/src/lib.rs]
crates: [update]
depends_on: [T004]
exclusive: []
estimated_minutes: 45
```

**taskmeta** (T007)

```yaml
id: T007
owner_files: [crates/update/src/lib.rs]
crates: [update]
depends_on: [T006]
exclusive: []
estimated_minutes: 30
```

**taskmeta** (T008)

```yaml
id: T008
owner_files: [crates/update/src/tests.rs]
crates: [update]
depends_on: [T007]
exclusive: []
estimated_minutes: 35
```

**taskmeta** (T009)

```yaml
id: T009
owner_files: [crates/update/src/arch.rs, crates/update/src/selection.rs]
crates: [update]
depends_on: [T001]
exclusive: []
estimated_minutes: 40
```

**taskmeta** (T010)

```yaml
id: T010
owner_files: [crates/update/src/arch.rs]
crates: [update]
depends_on: [T009]
exclusive: []
estimated_minutes: 25
```

**Checkpoint**: Configuration parses, the outcome model compiles, platform suffixes resolve. US1–US4
can now proceed.

**Note on file ownership**: T004 and T006 both touch `crates/update/src/lib.rs`, and T005/T008 both
touch `crates/update/src/tests.rs`. They are ordered, not parallel, for that reason.

---

## Phase 3: User Story 1 — Multiple Update Targets In One Configuration (Priority: P1) 🎯 MVP

**Goal**: A configuration names several targets; a check run reports each target's own status; the
legacy single-target file behaves exactly as before.

**Independent Test**: Configure a broken public target plus a private target; run the check; both
report independently and the run exits `2`. Separately, run against the legacy file and confirm
byte-identical behaviour to the pre-change build.

- [ ] T011 [US1] Implement `check_all` fan-out in `crates/update/src/command.rs`: iterate targets in configuration order, isolate each with its own `Result`, and always produce one `TargetCheck` per configured target (FR-008)
- [ ] T012 [US1] Implement exit-status derivation in `crates/update/src/command.rs`: `0` all clean, `1` configuration/usage error, `2` at least one target failed, per [contracts/update-cli.md](./contracts/update-cli.md) §2
- [ ] T013 [P] [US1] Add fan-out and exit-status tests in `crates/update/src/tests.rs` using `FixedReleases`: two targets both checked; first target failing still yields the second's real state; `n` targets yield `n` checks; exit `0` for an all-clean run, `1` for a configuration error and `2` for a partial failure (FR-026, SC-002)
- [ ] T014 [US1] Add legacy-compatibility test in `crates/update/src/tests.rs`: a flat single-target file produces the same outcome as the pre-feature call path (SC-001)
- [ ] T015 [US1] Update the CLI in `crates/cli/src/main.rs` so `guidance update check` calls the fan-out and renders one line per target, and `guidance update channel` without `--target` refuses a multi-target configuration by naming the targets

**taskmeta** (T011)

```yaml
id: T011
owner_files: [crates/update/src/command.rs]
crates: [update]
depends_on: [T007, T004]
exclusive: []
estimated_minutes: 40
```

**taskmeta** (T012)

```yaml
id: T012
owner_files: [crates/update/src/command.rs]
crates: [update]
depends_on: [T011]
exclusive: []
estimated_minutes: 20
```

**taskmeta** (T013)

```yaml
id: T013
owner_files: [crates/update/src/tests.rs]
crates: [update]
depends_on: [T011]
exclusive: []
estimated_minutes: 35
```

**taskmeta** (T014)

```yaml
id: T014
owner_files: [crates/update/src/tests.rs]
crates: [update]
depends_on: [T011, T013]
exclusive: []
sequencing_note: shares tests.rs with T013; not parallel
estimated_minutes: 25
```

**taskmeta** (T015)

```yaml
id: T015
owner_files: [crates/cli/src/main.rs]
crates: [cli, update]
depends_on: [T012]
exclusive: []
estimated_minutes: 40
```

**Checkpoint**: US1 delivers a working MVP — multiple targets are configurable and independently
reported, with the legacy path untouched. T013 and T014 both write `tests.rs`; run them in sequence
or split the file by test group.

---

## Phase 4: User Story 2 — Private Artifact Source Access Through GitHub Identity (Priority: P1)

**Goal**: A private artifact repository is readable with the user's GitHub identity, resolved from
the platform credential store; public targets keep working without a credential.

**Independent Test**: With a stubbed credential store holding a valid token, a private target
reports its offered version; with an absent entry the same target reports `inaccessible` and
installs nothing; a public target succeeds with no token consulted at all.

- [ ] T016 [US2] Add the `CredentialStore` port to `crates/update/src/ports.rs`: `fn get(&self, entry: &str) -> Result<Option<Secret>>` plus a `StoreUnavailable` error variant, keeping the existing boxed-future style only where async is required
- [ ] T017 [US2] Implement `crates/update/src/credentials.rs` with `cfg`-gated backends — Windows Credential Manager via `windows-sys` (`guidance:update:<entry>`), macOS Keychain and Linux Secret Service via process invocations — and a `cfg(not(any(windows, target_os = "macos", target_os = "linux")))` arm that returns the named unsupported-store condition (FR-011a)
- [ ] T018 [P] [US2] Add credential tests in `crates/update/src/tests.rs` with a `FixedCredentialStore` double: absent entry maps to `inaccessible`, an unusable store maps to `credential_store_unavailable`, and a present entry yields a secret that never appears in `Debug` output
- [ ] T019 [US2] Extend `Asset` in `crates/update/src/lib.rs` with `id`, and add an authenticated asset-reference type so the archive and its checksum are fetched through `repos/{repo}/releases/assets/{id}` with `Accept: application/octet-stream` (R1, FR-017)
- [ ] T020 [US2] Rework `crates/update/src/github.rs`: send `Authorization: Bearer` only when a token was supplied, use the asset endpoint for bytes, and map `401`/`403`/`404` to one indistinguishable `inaccessible` outcome (FR-013, FR-014)
- [ ] T021 [P] [US2] Update `crates/update/src/selection.rs` so archive and checksum resolve to the same authenticated reference, and add tests asserting both use it and that no `browser_download_url` is used for a private target
- [ ] T022 [US2] Wire credential resolution into the check path in `crates/update/src/command.rs`: private targets resolve the entry named by configuration, public targets never call the store (SC-008)

**taskmeta** (T016)

```yaml
id: T016
owner_files: [crates/update/src/ports.rs]
crates: [update]
depends_on: [T004]
exclusive: []
estimated_minutes: 30
```

**taskmeta** (T017)

```yaml
id: T017
owner_files: [crates/update/src/credentials.rs, crates/update/src/lib.rs]
crates: [update]
depends_on: [T016]
exclusive: []
estimated_minutes: 45
```

**taskmeta** (T018)

```yaml
id: T018
owner_files: [crates/update/src/tests.rs]
crates: [update]
depends_on: [T017]
exclusive: []
estimated_minutes: 35
```

**taskmeta** (T019)

```yaml
id: T019
owner_files: [crates/update/src/lib.rs, crates/update/src/ports.rs]
crates: [update]
depends_on: [T016]
exclusive: []
estimated_minutes: 35
```

**taskmeta** (T020)

```yaml
id: T020
owner_files: [crates/update/src/github.rs]
crates: [update]
depends_on: [T019]
exclusive: []
estimated_minutes: 45
```

**taskmeta** (T021)

```yaml
id: T021
owner_files: [crates/update/src/selection.rs]
crates: [update]
depends_on: [T019]
exclusive: []
estimated_minutes: 30
```

**taskmeta** (T022)

```yaml
id: T022
owner_files: [crates/update/src/command.rs]
crates: [update]
depends_on: [T020, T017]
exclusive: []
estimated_minutes: 30
```

**Checkpoint**: US2 makes a private source reachable. T019 and T021/T020 are sequenced because the
authenticated asset reference type is the shared contract they exchange.

---

## Phase 5: User Story 3 — Clear Outcome When Access Is Missing (Priority: P2)

**Goal**: An operator can tell an access problem from "up to date", a network failure and an invalid
artifact, from the output alone.

**Independent Test**: Produce all seven outcome states in one run against stubbed ports and confirm
every state is distinguishable in human and JSON output, and that the access report is identical for
an unauthorized identity and a non-existent repository.

- [ ] T023 [US3] Render the human-readable summary in `crates/update/src/report.rs`: one line per target, target name first, with the verbatim phrase `no access to update source` for `inaccessible` and a provisioning hint on a second line for an absent credential entry (SC-003)
- [ ] T024 [US3] Render the JSON document in `crates/update/src/report.rs` with `spec_version`, `checked_at`, `exit_status` and `targets[]`, serializing `state` as lowercase snake_case per [contracts/update-cli.md](./contracts/update-cli.md) §4 (SC-010)
- [ ] T025 [P] [US3] Add rendering tests in `crates/update/src/tests.rs`: all seven states produce distinct output; `network_error` and `invalid_artifact` never reuse the access phrase; the unauthorized and non-existent reports are identical (SC-005)
- [ ] T026 [US3] Add a secret-leak test in `crates/update/src/tests.rs` that runs every rendering path with a sentinel token value and asserts zero occurrences in text, JSON and error messages (NFR-4, SC-007)
- [ ] T027 [US3] Add `--format text|json` to the check command in `crates/updater/src/main.rs` and `crates/cli/src/main.rs`, defaulting to `text`

**taskmeta** (T023)

```yaml
id: T023
owner_files: [crates/update/src/report.rs]
crates: [update]
depends_on: [T004]
exclusive: []
estimated_minutes: 35
```

**taskmeta** (T024)

```yaml
id: T024
owner_files: [crates/update/src/report.rs]
crates: [update]
depends_on: [T023]
exclusive: []
estimated_minutes: 30
```

**taskmeta** (T025)

```yaml
id: T025
owner_files: [crates/update/src/tests.rs]
crates: [update]
depends_on: [T024]
exclusive: []
estimated_minutes: 40
```

**taskmeta** (T026)

```yaml
id: T026
owner_files: [crates/update/src/tests.rs]
crates: [update]
depends_on: [T024, T025]
exclusive: []
sequencing_note: shares tests.rs with T025; not parallel
estimated_minutes: 25
```

**taskmeta** (T027)

```yaml
id: T027
owner_files: [crates/updater/src/main.rs, crates/cli/src/main.rs]
crates: [updater, cli]
depends_on: [T024]
exclusive: []
estimated_minutes: 35
```

**Checkpoint**: US3 delivers diagnosability. T023 and T024 share `report.rs` and are ordered.

---

## Phase 6: User Story 4 — Installing an Update For Each Target (Priority: P2)

**Goal**: Applying an update for one target never disturbs another; the installed version comes from
a per-target marker; a failed install leaves the previous installation intact.

**Independent Test**: Install for the second target and verify the first target's files, marker and
service state are unchanged; remove a marker and confirm the target reports `not_installed`; force a
checksum mismatch and confirm the old version survives.

- [ ] T028 [US4] Create `crates/update/src/marker.rs` reading and writing `install.json` with `target`, `version`, `installed_at`; a missing, unreadable or target-mismatched marker resolves to `not_installed` (FR-009a–c)
- [ ] T029 [P] [US4] Add marker tests in `crates/update/src/tests.rs`: round-trip write/read; absent marker yields `not_installed`; foreign `target` value yields `not_installed`; the transitional legacy rule (no marker plus expected executable present ⇒ installed at the CLI version) is asserted and marked for later removal
- [ ] T030 [US4] Change `Installer::install` in `crates/update/src/ports.rs` to take the resolved target instead of the whole `UpdateConfig`, and adapt the runtime call site in `crates/update/src/lib.rs`
- [ ] T031 [US4] Refactor `crates/update/src/install.rs` into a shared transactional body taking a file set, an install directory and an optional service name; preserve stop → backup → replace → restart → restore-on-failure exactly for the runtime target, and skip service control when the name is absent (FR-020a, FR-021)
- [ ] T032 [P] [US4] Extend the existing install tests in `crates/update/src/install.rs` and `crates/update/src/tests.rs`: unsafe archive path still rejected, malformed archive still rejected, checksum mismatch leaves the install dir byte-identical and the marker unchanged, service-less target never invokes `sc.exe`
- [ ] T033 [US4] Write the version marker as the final step of a successful install in `crates/update/src/install.rs`, so a failure after replacement cannot claim the new version (FR-020b)
- [ ] T034 [US4] Add per-target install orchestration in `crates/update/src/command.rs`: `--target` narrows to one, omission applies every target reporting `update_available`, and `--yes` remains mandatory (FR-020, SC-004, SC-006)

**taskmeta** (T028)

```yaml
id: T028
owner_files: [crates/update/src/marker.rs, crates/update/src/lib.rs]
crates: [update]
depends_on: [T006]
exclusive: []
estimated_minutes: 35
```

**taskmeta** (T029)

```yaml
id: T029
owner_files: [crates/update/src/tests.rs]
crates: [update]
depends_on: [T028]
exclusive: []
estimated_minutes: 35
```

**taskmeta** (T030)

```yaml
id: T030
owner_files: [crates/update/src/ports.rs, crates/update/src/lib.rs]
crates: [update]
depends_on: [T028]
exclusive: []
estimated_minutes: 40
```

**taskmeta** (T031)

```yaml
id: T031
owner_files: [crates/update/src/install.rs]
crates: [update]
depends_on: [T030]
exclusive: []
estimated_minutes: 45
```

**taskmeta** (T032)

```yaml
id: T032
owner_files: [crates/update/src/install.rs, crates/update/src/tests.rs]
crates: [update]
depends_on: [T031]
exclusive: []
estimated_minutes: 40
```

**taskmeta** (T033)

```yaml
id: T033
owner_files: [crates/update/src/install.rs]
crates: [update]
depends_on: [T031]
exclusive: []
estimated_minutes: 25
```

**taskmeta** (T034)

```yaml
id: T034
owner_files: [crates/update/src/command.rs]
crates: [update]
depends_on: [T031, T012, T028]
exclusive: []
estimated_minutes: 35
```

**Checkpoint**: US4 completes the feature. T030 is the approved `Installer` signature change; it
must land as its own reviewable unit because it touches an existing trait.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: The operator decision of 2026-11-12 puts the macOS CI leg in scope as its own task, and
documentation must match behaviour.

- [ ] T035 Add a `macos-latest` entry to the matrix in `.github/workflows/ci.yml` so the platform claim in SC-009 is testable; leave the release matrix in `.github/workflows/release.yml` unchanged until the private product publishes non-Windows artifacts
- [ ] T036 [P] Update `config/updates.example.toml` to document both the legacy and the `[[targets]]` form, including `visibility` and `credential_entry`, with a comment stating the value is never stored there
- [ ] T037 [P] Update `docs/IMPLEMENTATION_STATUS.md`'s update chapter: multi-target fan-out, seven outcome states, authenticated private access, marker-based installed state, and the explicit statement that release matrix legs for macOS/Linux are deferred with their trigger
- [ ] T038 [P] Update `README.md` and `README.de.md` for the new configuration shape and the credential provisioning step, per the constitution's requirement that both languages move in the same change
- [ ] T039 Run [quickstart.md](./quickstart.md) scenarios 1–8 and 10 and record results; run scenario 9 on Windows and Linux, and record macOS as executed or as explicitly not executed
- [ ] T040 [P] Document the least-privilege requirement for the private artifact repository credential in `docs/IMPLEMENTATION_STATUS.md` — the provisioned token must grant read access to release artifacts of the configured source only, never to source repositories and never write access (FR-023) — and add the corresponding check to `quickstart.md` scenario 10

**taskmeta** (T035)

```yaml
id: T035
owner_files: [.github/workflows/ci.yml]
crates: []
depends_on: []
exclusive: [ci-workflow]
estimated_minutes: 15
```

**taskmeta** (T036)

```yaml
id: T036
owner_files: [config/updates.example.toml]
crates: []
depends_on: [T007]
exclusive: []
estimated_minutes: 20
```

**taskmeta** (T037)

```yaml
id: T037
owner_files: [docs/IMPLEMENTATION_STATUS.md]
crates: []
depends_on: [T034, T024]
exclusive: [docs/implementation-status]
estimated_minutes: 25
```

**taskmeta** (T038)

```yaml
id: T038
owner_files: [README.md, README.de.md]
crates: []
depends_on: [T036]
exclusive: []
estimated_minutes: 25
```

**taskmeta** (T039)

```yaml
id: T039
owner_files: []
crates: []
depends_on: [T034, T027, T035]
exclusive: []
estimated_minutes: 45
```

**taskmeta** (T040)

```yaml
id: T040
owner_files: [docs/IMPLEMENTATION_STATUS.md, specs/001-multi-target-update-sources/quickstart.md]
crates: []
depends_on: [T037]
exclusive: [docs/implementation-status]
estimated_minutes: 20
```

---

## Dependency Graph

```
T001 ─┬─ T002 (baseline, runs BEFORE T001)
      ├─ T003 (release contract doc, parallel)
      └─ T004 ─┬─ T005
               ├─ T006 ── T007 ── T008
               │    └──── T011 ─┬─ T012 ─┬─ T015
               │                ├─ T013  └─ T034
               │                └─ T014
               ├─ T016 ─┬─ T017 ── T018
               │        └─ T019 ─┬─ T020 ── T022
               │                └─ T021
               ├─ T023 ── T024 ─┬─ T025
               │                ├─ T026
               │                └─ T027
               └─ T028 ─┬─ T029
                        └─ T030 ── T031 ─┬─ T032
                                         ├─ T033
                                         └─ T034

T009 ── T010        (independent of T004–T008; blocks the platform claim, not the fan-out)
T035               (independent; CI only)
T036 ── T038       (after T007)
T037               (after T034, T024)
T039               (after T034, T027, T035)
T040               (after T037; shares docs/IMPLEMENTATION_STATUS.md)
```

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on T001 — **blocks every user story**
- **US1 (Phase 3)**, **US2 (Phase 4)**, **US3 (Phase 5)**, **US4 (Phase 6)**: each depends on
  Phase 2; US2 additionally requires T019's asset reference before US4's private install path is
  end-to-end
- **Polish (Phase 7)**: after the stories it documents

### Exclusive resources (serialization points)

| Resource | Owner tasks | Rule |
|---|---|---|
| `Cargo.toml` + `Cargo.lock` | T001 only | Any later dependency change must fold into T001 or open a new owned task |
| `crates/update/src/lib.rs` | T004, T006, T007, T019, T028, T030 | Serialized — never parallel with each other |
| `crates/update/src/tests.rs` | T005, T008, T013, T014, T018, T025, T026, T029, T032 | Serialized — T014 depends on T013, T026 on T025 |
| `crates/update/src/command.rs` | T011, T012, T022, T034 | Serialized |
| `crates/update/src/report.rs` | T004, T023, T024 | Serialized |
| `crates/update/src/install.rs` | T031, T032, T033 | Serialized |
| `crates/update/src/ports.rs` | T016, T019, T030 | Serialized |
| `.github/workflows/ci.yml` | T035 | Sole owner |
| `docs/IMPLEMENTATION_STATUS.md` | T003, T037, T040 | Serialized |

## Parallel Opportunities

**[P] marks correctness under file ownership, not narrative similarity.** Given the exclusivity
table above, the genuinely parallel groups are:

**Wave 1 (after the baseline)**
- T002 baseline evidence (runs **before** T001) · T003 release contract doc · T009 platform suffix table

**Wave 2 (after T004/T006)**
- T010 arch tests (needs T009) · T017 credential backends (needs T016) · T028 marker module

**Wave 3 (after the core modules exist)**
- T021 selection asset reference · T025 then T026 rendering tests (sequential — same file)
- T035 macOS CI leg at any time; it owns its file exclusively

Everything else is serialized by file ownership. Do **not** parallelize two tasks that share an
`owner_files` entry, even when their narrative sounds independent.

---

## Implementation Strategy

### MVP first (User Story 1)

1. Phase 1 (T001–T003)
2. Phase 2 (T004–T010) — the real gate; T009/T010 can run alongside T004–T008
3. Phase 3 (T011–T015)
4. **Stop and validate**: run quickstart scenarios 1 and 2. If the legacy path changed, stop —
   SC-001 is broken and no later story is worth building.

### Incremental delivery

1. Phase 1 + 2 → foundation ready
2. + US1 → multiple targets visible (MVP)
3. + US2 → private source reachable
4. + US3 → outcomes diagnosable and machine-readable
5. + US4 → installs are isolated, marker-backed and rollback-safe
6. Phase 7 → CI/legal claim alignment and documentation

### Sequencing constraints that are easy to get wrong

- **T030 before T031**: the `Installer` signature is the contract the refactor implements. Doing it
  the other way creates a transient duplicate install path, which Principle I treats as unsafe.
- **T033 after T031, not before**: the marker must be written by the transactional body, not by a
  caller that could succeed while the transaction failed.
- **T009 before T019/T020 on a non-Windows host**: without the widened suffix table, the adapter
  cannot be exercised at all on Linux or macOS and its tests would silently be Windows-only.
- **T035 before claiming SC-009**: the macOS leg is what makes the platform statement evidence-based
  rather than asserted.

## Notes

- [P] reflects file ownership, verifiable against the exclusivity table
- Every task names a file path an implementer can open without further context
- Constitution gates are reported in the PR, not in this file: `cargo fmt`, `cargo clippy`,
  `cargo test`, and both CI legs are the authoritative evidence (Principle II)
- The transitional legacy marker rule (T029) must be removed once every install path writes a
  marker; leave a comment naming that trigger at the removal site
