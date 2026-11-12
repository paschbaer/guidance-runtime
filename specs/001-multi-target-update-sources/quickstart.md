# Quickstart Validation — Multi-Target Update Sources

**Feature**: `001-multi-target-update-sources`
**Purpose**: Prove the feature end to end. This is a validation guide, not an implementation
guide — it names what to run and what must happen, and points at the contract for detail.

References: [`spec.md`](./spec.md) · [`data-model.md`](./data-model.md) ·
[`contracts/update-cli.md`](./contracts/update-cli.md) · [`research.md`](./research.md)

---

## Prerequisites

- A checkout of this repository with the feature branch.
- Toolchain: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (Constitution Principle II gates).
- For scenarios 4–7: a private artifact repository and a provisioned credential entry. Without
  them those scenarios are validated against the test doubles described in scenario 3 and marked
  as "not executed against a live source" — do not claim them as passed.

---

## Scenario 1 — Legacy configuration is unchanged (SC-001)

1. Use an `updates.toml` with only the legacy keys (see contract §5).
2. Run `guidance update check --config <path>`.

**Expected**: output and exit status identical to the pre-feature behaviour; a single line for
`guidance-runtime`; exit `0` when up to date.

**Regression signal**: any new required key, or a target list appearing in the output, is a
failure of FR-003.

---

## Scenario 2 — Two targets report independently (FR-008, SC-002)

1. Configure `guidance-runtime` (public) and `guidance` (private).
2. Break the public target only (point it at a non-existent repository).
3. Run `guidance update check`.

**Expected**: two lines, one per target. The public target reports `no access to update source`;
the private target still reports its real state. Exit `2`.

**Regression signal**: the run stopping after the first failure violates FR-008.

---

## Scenario 3 — Access outcomes are distinguishable (FR-015, SC-003, SC-005)

1. Run with `--format json` against a target with an unauthorized identity.
2. Repeat against a repository that does not exist.
3. Compare the two documents.

**Expected**: both report `state = "inaccessible"` and the identical human phrase
`no access to update source`; no field distinguishes absence from non-existence (FR-014).

---

## Scenario 4 — Missing credential (FR-013, SC-004)

1. Configure a private target with `credential_entry` set to an entry that is **not** provisioned.
2. Run `guidance update check --format json`, then `guidance update install --target guidance --yes`.

**Expected**: `state = "inaccessible"` with the provisioning hint; exit `2`. The install refuses
and **nothing** is written to the target's install dir (verify by listing the directory before and
after).

---

## Scenario 5 — Private artifact retrieval uses one access path (FR-017)

1. With a valid identity, run a check that reports `update_available`.
2. Run `guidance update install --target guidance --yes`.
3. Confirm through the updater's detail output or a network capture that the archive **and** the
   checksum were fetched through the authenticated API path, not `browser_download_url`.

**Expected**: install succeeds; a request to any `browser_download_url` never occurs.

---

## Scenario 6 — Checksum enforcement stays fail-closed (FR-018, FR-019, SC-007)

1. Point the target at a release whose checksum asset is absent → expect refusal, `invalid_artifact`.
2. Publish a release with a checksum that does not match the archive → expect abort, exit non-zero.
3. After both, compare the target's install dir with a pre-run listing.

**Expected**: identical listings in both cases; the previously installed version is intact and the
version marker still names the old version.

---

## Scenario 7 — Target isolation (FR-020, SC-006)

1. Install an update for `guidance`.
2. Inspect `guidance-runtime`'s install dir, its version marker and its service state.

**Expected**: all three unchanged. On Windows, confirm the runtime service was not stopped as a
side effect of the `guidance` install.

---

## Scenario 8 — Version marker drives the installed state (FR-009a–c)

1. Remove `install.json` from the target's install dir and run a check → expect `not_installed`.
2. Write a marker whose `target` names a different target → expect `not_installed` (FR-009c).
3. Perform a successful install → confirm the marker now names the new version.

**Expected**: `not_installed` is never reported as `up_to_date`, and a failed install never updates
the marker.

---

## Scenario 9 — Platform coverage (SC-009, R6)

| Platform | What to run | Expected |
|---|---|---|
| Windows | Scenario 1–8, `cargo test --workspace` | All pass |
| Linux | `cargo test --workspace` + scenario 1 and 3 with a public target | Pass; private credential path reports `credential_store_unavailable` when no Secret Service is present |
| macOS | `cargo test --workspace` (after the macOS CI leg exists) | Pass |

**Honesty rule**: until the macOS CI leg from R6 exists, do not claim macOS verification. Record
"not executed" explicitly — Principle II forbids a single-OS green being reported as cross-platform
success.

---

## Scenario 10 — No secrets in output and least privilege (NFR-4, SC-007, FR-023)

1. Run every command from the scenarios above with the token value captured in a shell variable.
2. Search all output, logs and error messages for that value.

**Expected**: zero occurrences. The `detail` field and the human lines are checked explicitly.

3. Inspect the provisioned token's scope. It must grant **read access to the release
   artifacts of the configured artifact repository only**:
   - `GET /repos/<artifact-repo>/releases` succeeds with the token;
   - a request against a **private** source-code repository the token is not meant to
     serve fails (401/403/404). If only a public repository is available, record the probe
     as inconclusive by design: a public read succeeds without any grant and proves nothing
     about the token's scope;
   - no write operation is attempted; the scope review is read-only.

**Expected**: the artifact read succeeds, the unrelated source repository is not readable,
and the token carries no write permission. Record the observed scope and the two request
results. A token that also grants source or write access is a provisioning defect, not a
pass.

---

## Definition of done for validation

- [x] Scenarios 1, 2, 4 and 8 executed on Windows and evidenced below (scenario 8's marker cases included)
- [x] Scenario 3 executed for the two-missing-source half; the unauthorized-identity half is **not executed** (no provisioned token) and is covered only by the classification unit test
- [ ] Scenarios 5 and 7 live runs: **not executed** — they require a provisioned credential and a live private artifact repository; their logic is covered by unit tests and is labelled as such. Scenario 6 is partially executed: the no-artifact cause was measured live, the missing-checksum cause is double-backed.
- [x] Scenario 9 executed on Windows (`cargo test --workspace`)
- [ ] Scenario 9 on Linux **not executed** and macOS **not executed** on this host; `macos-latest` was added to `ci.yml` but no CI run was available here
- [ ] Scenario 10 live secret scan and least-privilege scope probe: **not executed** — no token is provisioned; the zero-occurrence property is covered by the sentinel unit test
- [x] `cargo fmt`, `cargo clippy`, `cargo test` reported as run with results (below)
- [x] Every scenario validated only against test doubles is labelled as such

---

## Validation results — 2026-11-12, Windows (debug binaries)

Gates executed: `cargo fmt --all -- --check` clean, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` green (141 tests in `guidance-update`, 0 failures).

| Scenario | Result | Evidence / limitation |
|---|---|---|
| 1 — Legacy unchanged | **Executed** | Legacy text printed `No update available on stable channel`, exit `0`; `--format json` emitted one `guidance-runtime` document with `exit_status: 0`, exit `0`; a missing repository exited `1` with `no access to update source` and no document. The wording is pinned literally by `a_legacy_flat_configuration_produces_the_historical_outcome`; the pre-change binary was not rebuilt for this run. |
| 2 — Independent targets | **Executed** | First source broken, second healthy: `probe  no access to update source` followed by `healthy  up to date (1.1.0, stable)`, exit `2`. The run did not stop at the first failure. |
| 3 — Access outcomes distinguishable | **Partially executed** | Two non-existent public sources produced identical `state: "inaccessible"` and identical `detail: "no access to update source"` in JSON. The unauthorized-identity half needs a provisioned token and was **not executed**; `an_unauthorized_identity_and_a_missing_source_render_identically` covers the classification equality with doubles. |
| 4 — Missing credential | **Executed** | Check reported `state: "inaccessible"` with the provisioning hint (`guidance:update:qs-missing-entry`), exit `2`. The CLI's `guidance update install --target secret --yes` starts a detached updater and exits `0` on handoff; the spawned `guidance-updater --config … install --target secret --yes` reported `install failed: no access to update source …` and exited `2`. The install directory listing was empty before and after. |
| 5 — Private retrieval via one access path | **Not executed** | Requires a valid identity and a live private artifact repository. `selection_resolves_the_archive_to_the_authenticated_endpoint` and `an_authenticated_download_attaches_the_token_to_both_assets` cover the path with doubles. |
| 6 — Checksum enforcement | **Partially executed** | A live public repository with newer releases but no archive matching this platform reported `state: "invalid_artifact"` with exit `2`, so the state is reachable rather than collapsing to `up_to_date`. The missing-checksum cause was not reproduced live; it is covered by `a_release_without_a_checksum_is_invalid_artifact` and `install_all_refuses_an_unverifiable_release_without_installing`, and a mismatching checksum by `download_fails_on_a_mismatching_checksum` and `a_checksum_mismatch_leaves_the_install_dir_and_marker_unchanged`. |
| 7 — Target isolation | **Not executed live** | Requires installing a real target. `install_all_installs_only_update_available_targets` and `install_all_over_a_narrowed_configuration_touches_one_target` cover the per-target selection with doubles. The Windows service-state half (the runtime service is not stopped by another target's install) is **not covered by any automated test**: service control is not behind a port, so a test cannot intercept `sc.exe`. No real target was installed on this host. |
| 8 — Version marker | **Executed** | Marker `1.1.0` → `up to date (1.1.0, stable)`; no marker and no executable → `not installed`; `guidance-service.exe` without a marker → `up to date (1.1.0, stable)` (transitional rule); a marker naming another target plus the service executable → `not installed`; a foreign target with a valid marker → `up to date (1.1.0, stable)`. |
| 9 — Platform coverage | **Windows executed; Linux/macOS not executed** | `cargo test --workspace` green on Windows. No Linux or macOS host is available here; `.github/workflows/ci.yml` now includes `macos-latest`, but this session did not run CI. |
| 10 — No secrets / least privilege | **Partially executed** | No token is provisioned, so the live scan and the scope probe were **not executed**. `no_rendering_path_contains_the_credential_value` runs the real check pipeline with a sentinel and finds zero occurrences in text, JSON, `Secret` Debug/Display and `AuthenticatedAsset` Debug. The least-privilege statement is documented as a provisioning rule; the updater cannot verify token scope. |

Not executed is not passed. Every row above states which half was measured and which half is covered only by a test double.
