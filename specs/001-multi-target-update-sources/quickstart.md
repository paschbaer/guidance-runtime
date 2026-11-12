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

## Scenario 10 — No secrets in output (NFR-4, SC-007)

1. Run every command from the scenarios above with the token value captured in a shell variable.
2. Search all output, logs and error messages for that value.

**Expected**: zero occurrences. The `detail` field and the human lines are checked explicitly.

---

## Definition of done for validation

- [ ] Scenarios 1–8 executed and evidenced
- [ ] Scenario 9 executed on Windows and Linux at minimum; macOS recorded as executed or as not executed
- [ ] Scenario 10 executed with zero matches
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` reported as run with results
- [ ] Any scenario validated only against test doubles is labelled as such
