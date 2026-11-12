# Handover — Spec-Kit feature 001-multi-target-update-sources, T023 to T040

**Read this file first, then work through it without interruption.**

---

## 0. Operating instruction (binding)

**Work autonomously until all remaining tasks are complete.** Do not stop to ask questions. Do not
present options to the operator. Every decision that would otherwise be a question is instead:
made by you, evaluated with the Clear-Thought reasoning tools (`sequential_thinking`,
`decision_framework`, `socratic_method`, `debugging_approach`), recorded as a plan deviation with
its rationale, and carried on with.

The only exception is a genuine blocker: an infrastructure failure, a missing credential you cannot
obtain, or a contradiction in the specification that cannot be resolved by reasoning. In that case
use `report_blocker` and stop.

Report progress at batch boundaries in one short paragraph each. Do not produce a chat question
between batches.

---

## 1. Repository state

| | |
|---|---|
| Repository | `D:\repos\guidance-runtime` |
| Branch | `feature/multi-target-update-sources` |
| HEAD at handover | `c647fcf` |
| Workspace name | `guidance-runtime` (already registered) |
| Gates | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` |
| Current tests | 115 workspace, 98 in `guidance-update`, all green |

Commits so far: `c71cc22` (constitution, deliberately separate), `2d61aff` (multi-target fan-out,
T001-T015), `c647fcf` (private-source access, T016-T022).

---

## 2. How to run the work

Use the `guidance-speckit-tasks` skill. The feature is already imported; re-import to refresh.

```
start_workflow { workspace: "guidance-runtime", request: "<batch scope>" }
import_spec_kit_artifacts { featureId: "001-multi-target-update-sources" }
```

Per batch:

```
get_next_task
release_batch { mode: "explicit", batchId, taskIds }
start_task { batchId, taskIds }
  -> implement
submit_task_implementation { evidence: [one item per taskId] }
prepare_task_review { batchId }            # returns a token
spawn_agent { <fresh reviewer, review inputs only> }
submit_independent_review { reviewToken, findings }
verify_task / complete_task               # per task
```

Finish with `get_traceability_report`, `validate_spec_kit_completion`, `complete_workflow`.

### Batch partition (use this; it is ordered by file ownership)

| Batch | Tasks | Notes |
|---|---|---|
| batch-7 | T023, T024, T025, T026 | All `report.rs` + `tests.rs`. Ordered: T023 → T024 → T025 → T026. |
| batch-8 | T027 | `cli/main.rs`, `updater/main.rs`. Adds `--format`. |
| batch-9 | T028, T029, and the `check_one` change | Marker module **and its consumption**, see §4.1. |
| batch-10 | T030, T031, T033 | Ordered: T030 → T031 → T033. All `install.rs`/`ports.rs`. |
| batch-11 | T032, T034 | Tests plus install orchestration. |
| batch-12 | T035, T036, T037, T038, T040 | CI, example config, docs, both READMEs. |
| batch-13 | T039 | Quickstart validation. |

**Serial batch discipline**: `release_batch` re-points the session's active batch on every call and
nothing clears it. Finish every task of the active batch through `complete_task` before releasing
the next one, or the previous batch becomes unreachable and needs state surgery.

**Ownership rule learned the hard way in this feature**: a task that changes a compiled interface
must own *every* call site in the same batch, or the workspace does not build between tasks. T019
learned this: its first draft owned five files and broke three binaries. Before partitioning,
cross-check `taskmeta owner_files` against a `grep` for the symbol you are changing.

---

## 3. Binding constraints

These are not preferences. Violating one is a defect, not a deviation.

### 3.1 Two rendering paths must stay separate

`uses_legacy_output(config, target)` in `crates/update/src/command.rs` routes a legacy flat file
through the historical `UpdateOutcome::message` path; an explicit `[[targets]]` file goes through
`render_check` / `render_run`.

SC-001 requires a legacy flat configuration to produce **byte-identical** output and exit status.
T023 extends the existing renderers; it does not merge the paths.

Pinned historical strings, asserted by `a_legacy_flat_configuration_produces_the_historical_outcome`:

- `No update available on stable channel`
- `Update available: 1.1.0 -> 1.3.0-beta.10 (beta)`
- exit status **1** for a source failure

If a change appears to require merging the paths, record a plan deviation with
`propose_plan_change` and continue with the paths separate.

### 3.2 The inaccessible phrase and its hint

`report::PHRASE` is `"no access to update source"`. `render_check` prints it, then strips the
prefix from `detail` and renders the remainder on a second line:

```
guidance  no access to update source
  no credential named 'x' is provisioned on this machine; provision it as 'guidance-update:x'
```

Keep both. FR-013 requires the provisioning step; FR-013/FR-014 require the phrase to be identical
for unauthorized and for non-existent.

### 3.3 No secret may render

`Secret` has hand-written redacting `Debug`/`Display`; `AuthenticatedAsset` delegates to it;
`expose()` has exactly one production call site (`github.rs::authorize`).

Any new type holding a secret needs **its own** redaction test — a test on the inner type does not
cover a derived `Debug` on the outer struct. T026 must cover the JSON path too.

### 3.4 Fail closed

- No update without a matching SHA-256 asset.
- An absent credential entry and an unusable store must stay distinguishable: only the documented
  not-found exit status means absent.
- A failure must never be reported as `up_to_date`.
- **A run where every target failed must not exit 0.** This exact bug has occurred twice in this
  feature. Test it explicitly whenever you touch `check_one` or exit-status derivation.
- `classify_source_error` inspects the error by type, not by matching text.

### 3.5 Platform boundaries

`cfg`-gated code needs a compiling arm on every target. No gate in this repository exercises a real
credential store, a real Keychain, or a real Secret Service. Never claim otherwise, in code comments,
docs or the completion report.

---

## 4. Decisions already made — implement these, do not re-litigate

### 4.1 Version marker replaces the name heuristic (batch-9)

Today `check_one` decides the installed version from the target name: only `guidance-runtime`
reports the running crate version, every other target reports `not_installed`. Replace that with a
per-target marker.

**Marker**: `install.json` in the target's install directory, fields `target`, `version`,
`installed_at` (RFC3339).

- A missing, unreadable, or target-mismatched marker resolves to `not_installed`.
- The marker is written **last** in a successful install, so a crash after replacement cannot claim
  the new version.
- **Transitional rule, required**: no marker but the target's expected executable present in the
  install directory ⇒ `installed` at the running CLI's version. Without it, every existing runtime
  installation reports `not_installed` after the upgrade. Add a comment naming the removal trigger
  (every install path writes a marker) and a test pinning the rule.
- Introduce the module and consume it in the **same** batch. Leaving it unused between tasks
  produces dead code that a reviewer should rightly flag.

### 4.2 Service-less archives: the archive determines its content (batch-10, T031)

This was underspecified in the planning material. **Decision: option A.** Evaluated with
`sequential_thinking` against two alternatives; recorded here so it is not reopened.

- If the archive contains a `bin/` directory, only the contents of `bin/` are replaced. This is the
  current behaviour, so the runtime target is unaffected.
- Otherwise the archive root is replaced.
- "Replace" means copy and overwrite files. **Do not delete directories**: uninstalling is not part
  of this feature, and deleting could remove operator-placed files in the install directory.
- Service control is skipped when `target.service_name` is `None`; nothing else in the transaction
  changes.

Rejected alternatives and why: an explicit file list in configuration misplaces the decision in the
wrong artifact and a typo yields a partial install, which Principle I forbids; a manifest inside the
archive would require work in the release pipeline of the private product, which this repository
does not own, and a missing manifest would make installation impossible rather than permitted.

### 4.3 The JSON document is a separate output type (batch-7, T024)

`UpdateRun` has `exit_status()` as a method and knows no timestamp. The contract requires both as
**fields**. So build a dedicated serialisable output document carrying `spec_version`, `checked_at`,
`exit_status` and `targets[]`; do not derive `Serialize` onto `UpdateRun` and hope.

`checked_at` is RFC3339. `time 0.3.55` is already in the lock file transitively through
`toml_datetime`, so no new dependency is needed — verify before adding one.

A legacy flat configuration under `--format json` emits a one-element document. SC-001 constrains
the default output only, so this is consistent with the contract rather than an exception.

### 4.4 `--format` must not bypass the legacy path (batch-8, T027)

`--format json` changes the rendering only. It must not change the exit status, and it must not
route a legacy file through the per-target renderer. Default stays `text`, so the historical
behaviour is what an operator gets unless they ask otherwise.

---

## 5. Open findings — carry forward, do not silently fix

Recorded during earlier reviews. Fold one into a task only if that task already touches the file;
otherwise leave it and note it in the completion report.

| Item | Location |
|---|---|
| A proxy authentication challenge is classified as connectivity, not access. Needs a transport probe. | `github.rs` |
| `windows-sys` is pinned to 0.59 while other members resolve 0.61; a `cargo update` could change the FFI signatures behind the `unsafe` block. Bump deliberately and re-verify the block. | `Cargo.toml` |
| On Linux, `secret-tool` exits 1 for both a missing item and an unreachable daemon. | `credentials.rs` |
| A Windows blob provisioned as UTF-8 is mangled rather than rejected. | `credentials.rs` |
| Five configuration assertions check a substring rather than the pinned message. | `config_tests.rs` |
| No adapter-level test that a public target sends no `Authorization` header. | `tests.rs` |
| `AuthenticatedAsset` accepts an arbitrary URL with a token. | `ports.rs` |

---

## 6. What T035-T040 must get right

- **T035**: add `macos-latest` to `.github/workflows/ci.yml` **only**. The release matrix in
  `release.yml` stays Windows-only until the private product publishes non-Windows artifacts. Every
  future PR runs through this workflow, so a mistake there is felt immediately.
- **T036**: document both configuration forms in `config/updates.example.toml`. State that the first
  channel change on a legacy file **keeps it flat**.
- **T037, T040**: `docs/IMPLEMENTATION_STATUS.md` must describe what the code does, not what the plan
  wanted. It already names several limitations; keep them accurate rather than deleting them.
- **T038**: `README.md` and `README.de.md` in the **same** change. The constitution requires both
  languages to move together.
- **T039**: run `quickstart.md` scenarios 1-10. Scenarios needing a provisioned credential or a real
  private repository are recorded as **not executed**, never as passed.

---

## 7. Review protocol

Every code-bearing batch needs an independent review; `submit_task_review` is rejected for
self-review.

```
prepare_task_review { batchId }  -> token
spawn_agent { fresh context, review inputs only }
submit_independent_review { reviewToken, findings }
```

Give the reviewer the plan summary, the changed files, the relevant spec excerpts and the validation
commands. State explicitly what is out of scope so deferred work is not reported as missing. Ask it
to **run** the commands and to **measure** rather than read.

Two findings from this feature's history show why that standard matters:

- A reviewer built the **pre-change binary from `HEAD`** and measured both against identical configs.
  That is how it found a test pinning historical output on a function the CLI never called.
- A reviewer mutated a renderer and showed every test still passed, exposing an unguarded routing
  decision.

Press for that standard in the prompt you give each reviewer.

---

## 8. Definition of done

- [ ] T023-T040 complete, each verified and reviewed
- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace` green, reported as run
- [ ] `docs/IMPLEMENTATION_STATUS.md` and both READMEs match the code
- [ ] `memory-bank/activeContext.md` and `progress.md` updated
- [ ] `.guidance/state/final-review.json` written **after** the last commit and validated with
      `node .guidance/scripts/check-final-review.mjs`
- [ ] `.guidance/state/session-lessons.json` written (an empty array is a valid no-op)
- [ ] `validate_spec_kit_completion` reasoned about honestly: it requires all 40 tasks and every
      acceptance criterion. If a criterion cannot be evidenced in this environment, say so.
- [ ] Working tree committed; `develop` untouched

---

## 9. Reporting standard

At the end, state plainly:

- which tasks are complete and which are not, with evidence
- what was **measured** versus what was only read
- every finding accepted rather than fixed, with its rationale
- every claim the gates do not support, named as such

The most expensive defects in this feature were all of one kind: something looked green because
nothing tested the path a user actually takes. **A green suite is not evidence that the behaviour is
right.**
