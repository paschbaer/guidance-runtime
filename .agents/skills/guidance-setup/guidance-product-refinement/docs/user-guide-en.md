# User Guide: Guidance Product Refinement Skill

> **Version 2.0:** The skill now starts through one initial prompt. It collects missing placeholders in one grouped questionnaire and executes the PR/SH cycles internally. Manual copying of individual prompts is no longer required.

## Quick Start with the Initial Prompt

1. Load the skill.
2. Start `references/initial-workflow-prompt.md`.
3. If needed, answer the single grouped questionnaire.
4. The controller runs Product Refinement or Spec-Kit Handoff automatically.
5. Only when Spec-Kit must run externally, execute the one supplied copy-ready invocation and make its results available again.



## 1. Purpose of the Skill

The **Guidance Product Refinement** skill supports the staged specification of an entire product before the standard Spec-Kit workflow begins at feature level.

It provides two connected workflows:

1. **Product Refinement**
   - Product → Epic Set
   - Epic → Capability Set
   - Capability → Feature Candidate Set

2. **Spec-Kit Feature Handoff**
   - approved Feature Candidate
   - handoff to `/speckit.specify`
   - audit of the generated Feature Specification
   - automatic acceptance when there are zero blocking findings

The skill can be used as an interim solution while the equivalent workflow is not yet implemented directly in Guidance.

---

## 2. Core Concept

The skill does not follow the simple pattern:

```text
Idea → Specification → Tasks
```

Instead, it uses a controlled hierarchy:

```text
Product
  ↓
Epic Set
  ↓
Capability Set
  ↓
Feature Candidate Set
  ↓
Spec-Kit Feature Specification
```

Each transition consists of one or more controlled cycles:

```text
Generate → Audit → Revise → Audit again → Approve
```

Automatic approval occurs only when:

- all required audits have completed,
- no open blocking findings remain,
- the audited artifacts match the validated inputs,
- required digests and versions have been verified.

---

## 3. Installation

The skill package contains the following structure:

```text
guidance-product-refinement-skill/
├── SKILL.md
├── references/
│   ├── operator-guide.md
│   ├── product-refinement-prompts.md
│   └── speckit-feature-handoff-prompts.md
└── templates/
    ├── approval.yaml
    ├── findings.yaml
    └── run-manifest.yaml
```

Extract the ZIP archive into the skill directory of the target environment. The exact registration process depends on how skills are loaded in your agent or Guidance environment.

If automatic skill discovery is unavailable, the prompts can also be used directly from the files under `references/`.

---

## 4. Required Specification Subrepository

All specification artifacts managed by this skill should reside in a separate Git repository. It may be mounted in the main repository as a Git submodule.

Recommended structure:

```text
main-repository/
├── src/
├── tests/
├── .gitmodules
└── specification/
```

Inside the specification subrepository:

```text
specification/
├── product/
├── epics/
├── capabilities/
├── feature-candidates/
├── speckit/
│   └── features/
├── contracts/
├── hierarchy/
│   └── index.yaml
├── runs/
└── reviews/
```

The skill must not materialize specification artifacts outside this repository root.

---

## 5. Included Prompt Collections

### Product Refinement

The file `references/product-refinement-prompts.md` contains:

- **PR-1:** initialize or resume a refinement cycle
- **PR-2:** generate a complete Decomposition Proposal
- **PR-3:** audit the complete Child Set
- **PR-4:** revise the proposal from findings
- **PR-5:** evaluate the deterministic approval gate
- **PR-6:** materialize the approved Child Set

### Spec-Kit Feature Handoff

The file `references/speckit-feature-handoff-prompts.md` contains:

- **SH-1:** validate handoff eligibility
- **SH-2:** build the `/speckit.specify` input
- **SH-3:** capture and normalize the Spec-Kit result
- **SH-4:** audit the generated Feature Specification
- **SH-5:** generate a correction prompt
- **SH-6:** evaluate deterministic acceptance

---

## 6. Using Product Refinement

## 6.1 Prerequisites

A refinement run requires at least:

- the Parent Artifact or its repository path,
- the declared abstraction level,
- the requested target level,
- the specification-subrepository root.

The following are also recommended:

- the applicable Refinement Contract,
- the current hierarchy index,
- stable requirement IDs,
- Parent Artifact version and digest,
- a prior Run Manifest when resuming.

## 6.2 Supported Transitions

```text
product → epic-set
epic → capability-set
capability → feature-candidate-set
```

Skipping a hierarchy level is not allowed.

## 6.3 Starting the First Cycle

1. Open `product-refinement-prompts.md`.
2. Copy **PR-1**.
3. Replace all placeholders such as `{{SPEC_REPO_ROOT}}` and `{{PARENT_ARTIFACT_PATH_OR_CONTENT}}`.
4. Run the prompt with the agent.
5. Inspect the returned status.

Possible status values:

- `ready`: PR-2 may be executed.
- `clarification-required`: provide the specifically requested missing information.
- `superseded`: persisted inputs no longer match the current artifacts.
- `invalid`: the Parent, contract, or transition is invalid.

## 6.4 Generating the Decomposition Proposal

Run **PR-2** next.

The prompt generates a complete direct Child Set rather than one isolated child. For example:

```text
Epic: Workflow Orchestration
├── Capability: Task Routing
├── Capability: Execution Planning
├── Capability: Dependency Resolution
└── Capability: Retry Management
```

The output contains:

- proposal metadata,
- the complete Child Set,
- a Coverage Matrix,
- an Overlap Register,
- proposed new decisions,
- open questions.

The result is not yet approved and must not be treated as a final artifact set.

## 6.5 Auditing the Proposal

Pass the complete PR-2 result to **PR-3**.

The audit checks:

- complete coverage of the Parent Artifact,
- overlaps between children,
- child cohesion,
- correct abstraction level,
- scope preservation,
- propagation of constraints and non-goals,
- dependency integrity,
- traceability,
- ID conflicts,
- compliance with the Refinement Contract.

Findings use three severities:

- `blocking`: prevents approval,
- `warning`: does not prevent approval,
- `info`: observation or optional improvement.

## 6.6 Revision Cycle

If PR-3 reports blocking findings:

1. Run **PR-4**.
2. Supply the Parent, previous proposal, and complete findings.
3. PR-4 generates a complete new proposal revision.
4. Pass the revised proposal back to **PR-3**.
5. Repeat PR-3 and PR-4 until no blocking findings remain or the maximum cycle count is reached.

```text
PR-2
  ↓
PR-3 Audit
  ↓
Blocking findings?
  ├── Yes → PR-4 Revision → PR-3 Audit
  └── No  → PR-5 Approval Gate
```

Previous proposal and findings versions must not be overwritten.

Recommended storage:

```text
runs/<run-id>/proposal-v001.yaml
runs/<run-id>/proposal-v002.yaml
reviews/<run-id>/findings-v001.yaml
reviews/<run-id>/findings-v002.yaml
```

## 6.7 Automatic Approval

When PR-3 reports no open blocking findings, run **PR-5**.

Possible results:

- `auto-approved`: materialization is permitted.
- `rejected`: further revision is required.
- `pending-verification`: deterministic verification data is missing.

`pending-verification` typically occurs when the agent cannot calculate real digests or inspect the file-system state. The listed checks must then be completed with suitable tools.

Warnings and informational findings do not prevent `auto-approved`, but remain in the audit trail.

## 6.8 Materializing the Artifact Set

After `auto-approved`, run **PR-6**.

PR-6 creates or prepares:

- final Child Artifacts in English Markdown,
- YAML frontmatter,
- the updated hierarchy index,
- the findings file,
- the audit report,
- the approval record,
- a staging manifest containing target paths and digests.

The prompt may report `materialization-complete` only if the files were actually written and verified.

---

## 7. Resuming Refinement Later

A later cycle starts again with **PR-1**.

Also provide:

- the latest Run Manifest,
- the current Parent Artifact,
- the pinned Parent version and digest,
- the contract version,
- the latest proposal,
- unresolved findings.

If the Parent or contract changed, the old run must not continue silently. It must be superseded or replaced by a new run.

---

## 8. Using the Spec-Kit Feature Handoff

## 8.1 Prerequisites

The handoff requires:

- an approved Feature Candidate,
- a valid approval record,
- no open blocking findings,
- the `feature-to-speckit` contract,
- an available Spec-Kit invocation mechanism,
- an output path inside the specification subrepository.

## 8.2 Validating Handoff Eligibility

Run **SH-1**.

Possible results:

- `eligible`: SH-2 may be executed.
- `ineligible`: the reported issues must be resolved first.
- `pending-verification`: approval, digest, or path safety could not be verified deterministically.

## 8.3 Building the Spec-Kit Prompt

Run **SH-2**.

The prompt produces a copy-ready English input for:

```text
/speckit.specify
```

It contains:

- Feature Candidate ID and version,
- problem and desired outcome,
- source requirements,
- constraints,
- non-goals,
- scenarios and edge cases,
- clarification items,
- traceability requirements,
- a prohibition on unauthorized scope expansion.

## 8.4 Running Spec-Kit

Execute the prompt generated by SH-2 in the intended Spec-Kit environment.

Capture, where available:

- generated files,
- Spec-Kit version,
- command or adapter used,
- exit status,
- standard output and standard error,
- start and end timestamps.

## 8.5 Capturing the Result

Run **SH-3**.

SH-3:

- verifies execution,
- inventories generated artifacts,
- checks the repository boundary,
- identifies the primary Feature Specification,
- normalizes the output,
- records digests and tool metadata.

The result is not accepted at this stage.

## 8.6 Auditing the Feature Specification

Run **SH-4**.

The audit compares the generated Feature Specification with the approved Feature Candidate and checks:

- coverage of every source requirement,
- preservation of constraints and non-goals,
- scenarios and edge cases,
- unauthorized scope expansion,
- unsupported assumptions,
- internal consistency,
- testable acceptance criteria,
- traceability,
- contract conformity.

## 8.7 Correction Cycle

If SH-4 reports blocking findings:

1. Run **SH-5**.
2. Apply the generated correction prompt in Spec-Kit.
3. Run SH-3 again.
4. Run SH-4 again.
5. Repeat until no blocking findings remain or the maximum cycle count is reached.

```text
SH-2
  ↓
Run Spec-Kit
  ↓
SH-3 Capture
  ↓
SH-4 Audit
  ↓
Blocking findings?
  ├── Yes → SH-5 Correction → Spec-Kit → SH-3 → SH-4
  └── No  → SH-6 Acceptance Gate
```

## 8.8 Automatic Acceptance

Run **SH-6** when SH-4 reports zero blocking findings.

Possible results:

- `auto-accepted`: the specification may be materialized as final.
- `rejected`: correction is required.
- `pending-verification`: deterministic evidence is missing.

Acceptance does not authorize task generation or implementation. It only confirms that the Spec-Kit Feature Specification conforms to the approved Feature Candidate.

---

## 9. Templates

### `run-manifest.yaml`

Stores, among other data:

- Run ID,
- workflow type,
- state,
- cycle number,
- source artifact,
- contract version,
- digests,
- completed stages,
- generated outputs.

### `findings.yaml`

Stores:

- findings from the current audit,
- finding status,
- counts of open findings by severity.

### `approval.yaml`

Stores:

- approval ID,
- automatic or pending approval,
- referenced digests,
- contract version,
- audit status,
- blocking and non-blocking finding counts.

---

## 10. Practical Recommendations

- Start with one representative Epic and take it through every level.
- Introduce stable requirement IDs in the Product Artifact.
- Version Refinement Contracts independently from artifacts.
- Commit each approved refinement step separately.
- Preserve rejected proposals and historic findings.
- Use separate agent runs for authoring and audit where possible.
- Limit rework cycles, for example to five iterations.
- Verify digests, path boundaries, and Git state with actual tools rather than prompts alone.
- Never treat `pending-verification` as approval.

---

## 11. Common Mistakes

### Generating Individual Children Instead of a Complete Set

This prevents reliable coverage and overlap analysis. Always generate the complete direct Child Set.

### Using Spec-Kit Too Early

Spec-Kit should receive an approved Feature Candidate, not an unstructured product idea.

### Discarding Warnings

Warnings do not block approval, but they must remain in the audit trail.

### Claiming Approval in Prose

An agent must not claim approval solely because a specification appears plausible. The deterministic gate rules apply.

### Overwriting Previous Cycles

Every revision receives a new version or cycle number.

### Writing Outside the Subrepository

All specification, review, and run artifacts must remain inside the configured specification subrepository.

---

## 12. Quick Start

```text
1. Select the Parent Artifact
2. Run PR-1
3. Run PR-2
4. Run PR-3
5. If findings exist: repeat PR-4 → PR-3
6. Run PR-5
7. If auto-approved: run PR-6
8. For a Feature Candidate: run SH-1
9. Run SH-2
10. Run /speckit.specify
11. Run SH-3 and SH-4
12. If findings exist: SH-5 → Spec-Kit → SH-3 → SH-4
13. Run SH-6
```

This process enables controlled, iterative, and traceable hierarchical product specification before the native Guidance implementation is available.