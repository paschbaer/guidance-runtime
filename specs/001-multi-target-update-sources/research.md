# Research — Multi-Target Update Sources

**Feature**: `001-multi-target-update-sources`
**Phase**: 0 (Outline & Research)
**Date**: 2026-11-12

This document resolves the open questions carried over from the specification and records the
decisions the plan rests on. Each entry follows Decision / Rationale / Alternatives.

---

## R1. Authenticated release retrieval for a private repository

**Decision**: Read the release list and download artifacts through the GitHub REST API with an
`Authorization: Bearer <token>` header. Artifact bytes are fetched from the release-asset endpoint
(`repos/{owner}/{repo}/releases/assets/{id}`) with `Accept: application/octet-stream`, not from
`browser_download_url`.

**Rationale**: `browser_download_url` is served without authentication and returns `404` for a
private repository. It is the single reason the current adapter cannot reach a private source at
all. The asset endpoint accepts a token and streams the same bytes, and it is the only documented
path that keeps the token out of a redirect to unauthenticated storage.

**Constraint discovered**: `Asset` in `crates/update/src/lib.rs` currently carries only `name` and
`browser_download_url`. The asset endpoint needs `id`. The checksum artifact needs the same
treatment, so both asset references must be resolved the same way — otherwise the update check
succeeds while the checksum fetch fails with `404`, which is exactly the partial-access trap the
spec's FR-017 forbids.

**Alternatives considered**:
- *Install GitHub CLI and shell out to `gh release download`*: rejects on the constitution's
  port-inversion requirement and on testability; introduces a second binary and its own auth state.
- *Read `browser_download_url` and append a token query parameter*: GitHub does not accept tokens
  in the query string for this endpoint, and it would leak the token into logs and referrers.
- *Use a `codeload`/archive URL*: serves a source tarball, not the compiled release asset.

---

## R2. Platform credential store

**Decision**: Resolve the token from the platform's native per-user secret store behind a single
`CredentialStore` port with three implementations:

| Platform | Backend | Access method |
|---|---|---|
| Windows | Credential Manager | `CredWrite`/`CredRead` via `windows-sys` (`CRED_TYPE_GENERIC`, target `guidance:update:<entry>`) |
| macOS | Keychain | `security find-generic-password -s guidance-update -a <entry> -w` (process call, no new C dependency) |
| Linux | Secret Service (freedesktop) | D-Bus `org.freedesktop.secrets` lookup for `service=guidance-update`, `username=<entry>` |

**Rationale**: Each backend keeps the secret under the OS's per-user protection, which satisfies
FR-011 without inventing a file format. Windows uses a direct API call because the crate tree
already builds against `windows-sys`-class dependencies and a direct call avoids spawning a
process for the common platform. macOS and Linux use process calls because the D-Bus/Keychain
FFI surface would add build-time dependencies on non-Windows CI legs for no functional gain.

**Key question — why "one entry per platform" is not a behavioural difference**: the configuration
names one credential entry (`credential_entry = "guidance"`). The platform decides *where* that
entry lives. This is what FR-011b requires: the store is an implementation detail, the entry name
is not.

**Absent vs. unsupported**: a store that is unavailable (headless Linux without a Secret Service
daemon, or a locked keychain) is reported as a distinct named condition, never as "credential
absent" and never as a silent plaintext fallback. FR-011a and the spec's edge cases require the
two to be distinguishable.

**Alternatives considered**:
- *`keyring` crate*: wraps the same three backends but pulls a large dependency tree and its Linux
  feature selection is compile-time; a hand-written port keeps the platform boundary explicit as
  Principle V demands.
- *Encrypted file with DPAPI only*: Windows-only; contradicts the operator's explicit requirement
  that macOS and Linux are in scope.
- *Environment variable*: rejected by the constitution's "secrets MUST NOT appear in TOML files"
  reasoning extended to the process environment, and it is inherited by every child process.

---

## R3. Version marker format and placement

**Decision**: A per-target marker file named `install.json` (not `.version` — the file will grow)
inside the target's installation directory, containing JSON `{ "target": "<name>", "version":
"<semver>", "installed_at": "<rfc3339>" }`.

**Rationale**: JSON survives the addition of fields without a format migration, and the crate tree
already depends on `serde_json` transitively through `serde`. The marker is written **after** the
replacement transaction commits and is the last step of a successful install (FR-020a), so a crash
between replacement and marker write leaves the old marker in place and the next run reinstalls —
the safe direction.

**Reading rule**: a missing marker, an unreadable marker, or a marker whose `target` field does not
match the configured target name all resolve to **not installed** (FR-009b). A marker that parses
but names a different target is treated as absent rather than as a version mismatch, because
reusing another target's version would silently suppress a real update.

**Migration**: the runtime target has no marker today. Its absence is reported as "not installed"
on first run after the change, which would be misleading for a machine that clearly has the
runtime. The plan therefore seeds the marker during `install` only; the check handles the legacy
case by treating "no marker but `guidance-service.exe` present in the install dir" as installed at
the CLI's own version. This is recorded in `data-model.md` as a transitional rule and must be
removed once every supported install path writes a marker.

**Alternatives considered**:
- *Ask the binary for its version*: requires executing installed code during a check, which is
  unsafe for an unreviewed artifact and impossible for a target that is not currently runnable.
- *Store the version in `updates.toml`*: mixes operator intent with observed state, and the file
  is rewritten by `set_or_get_channel`, so the version would drift.

---

## R4. Per-target installed-version and update-check orchestration

**Decision**: `check_all(&config, &ports)` fans out over targets sequentially, collecting a
`Vec<TargetCheck>` where each entry is `Result`-independent: one target's failure produces a
`TargetOutcome::Inaccessible` (or the matching failure variant) entry and the loop continues.

**Rationale**: Sequential fan-out keeps the run deterministic and the output order stable, which
matters because the structured output is an automation contract (SC-010). Concurrency would add
no value at a handful of targets and would complicate per-target error attribution.

**Exit status**: `0` = every target up to date or successfully checked with no update; `1` = usage
or configuration error (no target checked); `2` = at least one target had a failure outcome while
others completed. This three-way split is what makes SC-003 workable from a script.

**Alternatives considered**:
- *Concurrent fan-out with `join_all`*: rejected; non-deterministic ordering complicates both the
  human summary and the golden tests.
- *Fail the whole run on first error*: directly contradicts FR-008.

---

## R5. Installation for the service-less target

**Decision**: Factor the existing transactional body out of `install.rs` into a reusable
`TransactionalInstall` that takes a list of files to replace, an install dir and an optional
service name. The runtime target passes four binaries and `service_name`; the private target
passes its artifact's file set and `None`.

**Rationale**: The existing body already implements exactly the sequence the spec requires for
both targets (backup → replace → restart-or-skip → restore on failure). Duplicating it would
duplicate the rollback bug surface, and the constitution's Principle IV requires dependencies to
be inverted through ports rather than copied.

**Behaviour with no service**: the stop/start steps are skipped, but the backup, the restore on
failure, and the "marker only on success" ordering are unchanged (FR-020a).

**Alternatives considered**:
- *Separate installer type per target*: acceptable, but it duplicates the rollback path that the
  constitution treats as safety-critical.
- *Extract-and-replace entire directory*: rejected; it discards operator-placed files and makes the
  backup/restore comparison ambiguous.

---

## R6. Platform asset selection on macOS and Linux

**Decision**: Widen `arch.rs` from "Windows suffixes" to a `(platform, arch) -> suffix` mapping;
`KNOWN_ARCH_SUFFIXES` becomes platform-aware. On macOS and Linux, the updater resolves
`macos-aarch64.zip`, `macos-x86_64.zip`, `linux-x86_64.zip`, `linux-aarch64.zip`.

**Rationale**: This is the hard blocker found during research. `asset_suffix_for_arch` currently
matches only `x86_64`/`aarch64` **and returns Windows-only suffixes**, so on a Linux host the
update path fails before any network call. The spec now puts all three platforms in scope, and
Constitution Principle V states that a hardcoded default is a defect and that a new architecture
requires an entry in `KNOWN_ARCH_SUFFIXES` plus a release matrix leg.

**Consequence for verification**: SC-009 asks for an executed check per platform. The current
matrix is `windows-latest` + `ubuntu-latest`, so macOS is the gap. The plan records two options
and selects the honest one:

- **Selected**: add a macOS leg to `ci.yml` and keep the platform claim to check/install logic. The
  release matrix needs macOS/Linux legs **only when the private product actually ships those
  artifacts**; until then the suffix table covers them but the release workflow does not publish,
  and the updater reports the absence as "no matching artifact", not as a crash.
- *Rejected*: claiming all three platforms without a macOS CI leg; Principle II forbids calling a
  single-OS green a success.

**Hotfix note**: the current `asset_suffix_for_arch` return values (`windows-x64.zip`) are used by
`select` to compare against asset names. Widening must keep the Windows mapping byte-identical so
existing installations do not start reporting "no matching artifact".

---

## R7. Structured output contract

**Decision**: A single `--format json` flag on the check command, emitting one JSON document with
`{ "run": {...}, "targets": [ {...} ] }` where each target object carries `name`, `state`,
`installed_version`, `offered_version`, `channel`, `detail`. `state` is a closed enum serialized
lowercase-snake: `update_available`, `up_to_date`, `not_installed`, `inaccessible`,
`network_error`, `invalid_artifact`, `credential_store_unavailable`.

**Rationale**: A closed enum is what makes the four outcome families machine-distinguishable
(FR-015, SC-003, SC-010). `credential_store_unavailable` is kept separate from `inaccessible`
because the remedy differs (provision a store vs. get access), and the spec's edge cases require
the distinction while FR-014 forbids distinguishing unauthorized from non-existent.

**Alternatives considered**:
- *Reuse the human string and let scripts regex it*: rejected; it makes SC-010 unverifiable.
- *Emit one line per target (ndjson)*: viable, but a single document is simpler to assert in tests
  and to consume from PowerShell.

---

## R8. Release mirroring

**Decision**: The private product's release workflow gains a mandatory "publish to artifact
repository" job that runs after artifact verification and fails the workflow when the mirror step
fails. Its implementation lives in the private product's repository, **not** in this one; this
plan only fixes the contract (artifact naming, checksum asset presence) the updater depends on.

**Rationale**: FR-027–FR-029 are pipeline requirements, but the pipeline being changed is not in
this workspace. Recording it as an external contract keeps this feature's scope honest and gives
`tasks.md` a verifiable acceptance step ("the mirror contract is documented and the consuming
naming rule matches it").

**Alternatives considered**:
- *Implement the mirror in this repository's `release.yml`*: wrong repository; this one publishes
  the runtime, not the private product.

---

## Resolved unknowns

| Spec open question | Resolved by |
|---|---|
| Authenticated download path | R1 |
| Which credential mechanism, per platform | R2 |
| Expired vs. absent credential | R2 (distinct `credential_store_unavailable` vs. `inaccessible`) |
| Version marker format, location, migration | R3 |
| Rollback semantics for the service-less target | R5 |
| Is the target list checkable as a whole or per target | R4 (fan-out over all; per-target status inside) |

**Remaining for planning-time judgement, not research**: the macOS release matrix leg (R6) is a
decision recorded here; whether the private product publishes macOS/Linux artifacts at all is
owned by that product's pipeline.
