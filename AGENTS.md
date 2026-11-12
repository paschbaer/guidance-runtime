# AGENTS.md

## Working Rules (Hard Rules)
- **Strict Compliance**: Strictly adhere to all guidelines in AGENTS.md. Any deviation requires explicit user approval: (1) explain the reason, (2) request explicit consent, (3) document the approved deviation in AGENTS.md. This rule takes precedence over all others when conflicts arise.
- **No Unrequested Changes**: After findings/analyses, the response is observation + solution options. Any corrective action — config edits, registry changes, file changes outside the given task — first requires a concrete confirmation from the user ("Should I do X?") and the agent waits. Ambiguity resolves to NOT acting. This overrides proactivity defaults.
- **Ask, Don't Assume**: When requirements are unclear or a decision is pending, ask the user concretely: present the available action options with their trade-offs, name a recommendation, and wait for the user's answer before continuing. Do not decide unilaterally.
- **File Deletion**: Ask for approval before deleting files you haven't created yourself.
- **Communication**: Keep explanations concise.
- **Language**: Communicate with the user in **German**. All artifacts — code, identifiers, comments, documentation, commit messages, prompts, configs, and generated files — are ALWAYS written in **English**.

## SOLID Principles (binding)
- **SRP — Single Responsibility:** one module/class = one responsibility and one reason to change. Generalizes the existing rule "keep strategy logic isolated by module responsibility"; applies to every `servers/*` project (e.g. state holding, (de-)serialization and reporting are separate concerns).
- **OCP — Open/Closed:** extend via registration/spec lists and dispatchers (pattern: `ToolsetRegistry` in server-clear-thought), not by modifying working dispatch logic.
- **LSP — Liskov Substitution:** every implementation of an interface must be substitutable without behavior surprises (same pre-/post-conditions, no narrower input contracts).
- **ISP — Interface Segregation:** dependents receive narrow interfaces with what they use — not a fat facade exposing far more than the dependent needs.
- **DIP — Dependency Inversion:** high-level modules depend on ports/interfaces; concrete adapters are wired at the composition root (patterns: `StorageAdapter` in server-insight, `ado-work-item-port` + `http-adapter` in server-azuredevops).
- **Scope of the obligation:** binding for NEW code and code under active modification. Pre-existing violations are NOT fixed proactively (NO UNREQUESTED CHANGES): register them per Findings Lifecycle Rule (activeContext.md + remaining-work-plan.md) with a trigger point, and fix them at that trigger.
- **Review criterion:** SOLID violations found in review are findings like any other; MEDIUM+ severity violations must be fixed or explicitly classified with evidence before merge (see Review Evidence Protocol).

## Planning & Reasoning
- Break down complex tasks into a plan first.
- For complex tasks, architectural decisions, or refactoring requests, break the problem into logical steps, verify assumptions, and identify edge cases BEFORE writing code or modifying files. Document the thought process in at least 3-5 steps; revise the plan if it seems uncertain.
- **Bugfix Protocol** — when investigating and fixing bugs, reasoning steps MUST include:
  1. **Reproduction:** Describe exactly how to reproduce the bug; create a failing test case first if possible.
  2. **Root Cause Analysis:** Explain *why* the bug happens, not just *where*.
  3. **Impact Assessment:** Check whether the bug or fix affects other parts of the system.
  4. **Fix Strategy:** Compare at least two approaches (e.g., "quick fix" vs. "robust refactor") before choosing one.
  5. **Verification Plan:** Define how the fix is proven (e.g., "Run npm test", manual UI check).
- **Baseline-aware testing:** Run focused tests first and label pre-existing full-suite failures separately to avoid attributing unrelated regressions to the current task.

## Architecture
- Keep responsibilities isolated by clear module boundaries.
- A strict layered architecture is the goal: each layer may only access the layer directly beneath it, never skip layers or reach upwards.
- Extend this section with project-specific implementation rules — see the prompt `.github/prompts/fill-agents-md.prompt.md`.

## 4. Git & Branch Management
- **Feature branches:** When working on `main` or `develop`, always create a feature branch `feature/<meaningful-name>`; all code changes go there (use working trees for concurrent changes), never directly on `main`/`develop`.
- **Before merging:** perform a thorough code review; run all tests and verify error-free execution.
- **Merging:** prefer rebase when merging into `develop`; use squash commits when merging into `main`.
- **After merge:** delete the feature branch.
- **Commits:** clear, descriptive messages following conventional commits format.
- **Cleanup:** remove temporary files before committing (ask for approval before deletion).
- **Documentation:** update README.md for new features or configuration changes and keep documentation in sync with code.

## Memory Bank
Durable project knowledge in `memory-bank/`:

| File | Content |
|---|---|
| `projectBrief.md` | Project purpose, scope, users, success criteria |
| `productContext.md` | User needs, workflows, domain language, business rules |
| `systemPatterns.md` | Architecture, design patterns, boundaries, engineering conventions |
| `techContext.md` | Technology stack, tooling, environments, operational constraints |
| `activeContext.md` | Current focus, recent decisions, open questions, next actions |
| `progress.md` | Status of completed, active, and planned work |
| `lessonsLearned.md` | Reusable lessons from incidents, mistakes, experiments, successes |
| `remaining-work-plan.md` | Tracked follow-ups from review findings and deferred work |

**Before a task:** read `projectBrief.md`, `systemPatterns.md`, `techContext.md`, `activeContext.md`, `progress.md`; consult `productContext.md` for user-facing/domain changes; review `lessonsLearned.md` for prior experience.

**After substantial work:**
- Update `activeContext.md` with new state and next actions; also after every significant change.
- Update `progress.md` ("Overall Status", "Completed", "In Progress", "Planned") — required BEFORE marking a task completed or ending a session; provide a final summary in chat afterwards.
- Update `systemPatterns.md` when architectural decisions change.
- Update `lessonsLearned.md` when resolving a recurring bug, a recurring failing command, a difficult bug, a clever optimization, or a strategic architectural decision. Focus on *why* things failed and *how* to do them right next time.
- After a confirmed bugfix, reflect on whether it is a recurring pattern or a non-obvious trap; if so, add a concise entry ("issue, root cause, preventive measure") to the "Avoid These Mistakes" section of `lessonsLearned.md`.

**Maintenance rules:** Keep entries concise, factual, and current; don't duplicate source code or generated docs; link ADRs/issues/PRs/source files where useful; mark assumptions and unresolved questions explicitly; never store credentials, tokens, personal data, or secrets; remove obsolete information instead of letting contradictions accumulate. These files are the source of truth — always keep them maintained.

## Code Review Protocol
- **Reviewer requirement:** Reviews MUST be performed by the `Review Agent` (`.github/agents/review.agent.md`) as a subagent with a fresh context window (see also the checklist in `.github/prompts/code-review-checklist.prompt.md`). Do not review directly in the main conversation.
- **Before reviewing:** verify `git status --short --branch`, `git rev-parse HEAD`, `git diff`, `git diff --cached`, and the exact review scope; a changed snapshot invalidates the review. For post-commit reviews, verify the target commit with `git show <commit> --stat` and inspect its file diff directly.
- **Evidence:** never dismiss a HIGH/CRITICAL finding without verifying it against the current source and a reproducible check. Record a concise evidence table per finding: file/symbol; reproducible execution path; current code location; test/direct check that proves or disproves it; whether the current diff introduced it; actual severity. One evidence-table row per finding.
- **Approval:** a review may be marked approved only after every HIGH/CRITICAL finding is either fixed or explicitly classified with evidence (already fixed, pre-existing, out of scope with a tracked follow-up, or verified false positive). Never output "approved" without an explicit count of unresolved HIGH/CRITICAL findings.
- **Output:** include a snapshot table stating branch, HEAD, review basis, staged/unstaged diff status, current-source reads, and tests actually executed.
- **Conflicting reports:** treat current source plus reproducible test/check as authoritative over the agent's claim. Stale or contradictory reviewer reports must be recorded as review-quality issues — without removing the obligation to classify the underlying technical concern separately.

## Findings Lifecycle
- Every unresolved review finding (any severity) must be persisted as a tracked follow-up in `memory-bank/activeContext.md` AND `memory-bank/remaining-work-plan.md` BEFORE the scope is closed or the session ends. Reviewer reports and chat summaries do not count as documentation.
- Each entry must state: the finding; its trigger point (the concrete future scope, stage, or condition under which it must be handled); and whether action is required or it is an accepted observation with rationale.
- When a future scope begins, its owner must check tracked follow-ups for matching trigger points and either handle them or explicitly re-schedule them with a new trigger point.
- A finding may only be removed when it is fixed with regression coverage or explicitly reclassified with evidence (e.g., verified false positive or subsumed by another change).

## Rule Updates & Self-Evolution
- **Rule updates:** BEFORE modifying AGENTS.md or any rule file: (1) back it up to `AGENTS.md.bak`, (2) verify the new rules don't contradict existing ones, (3) state in chat what changes are being made and why. If an update fails or causes logic loops, immediately offer to restore from the `.bak` file.
- **Self-evolution:** If a reasoning error or incomplete planning occurs, proactively suggest a Memory Bank Protocol update. After completing a complex task, analyze whether the existing rules were sufficient; if not, ask "Should I optimize the Memory Bank Protocol to avoid this mistake in the future?". The agent is authorized to propose new best practices discovered during work as permanent rules for future sessions.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **guidance-runtime**.

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact before editing.** Use `impact({target: "symbolName", direction: "upstream"})` or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .`; report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
- MUST warn on HIGH/CRITICAL `risk` pre-edit; never use `riskSharedAxes` to waive a HIGH/CRITICAL `risk` warning. Compare File/symbol: MCP File omits axes; Graph-RAG expands File.
- **MUST treat `risk: UNKNOWN` as unresolved, not as low.** An empty caller set is not evidence the symbol is unused — it can also mean the callers are not resolvable by the index (plain-object property access, dynamic dispatch, cross-language calls). `impact` pairs `UNKNOWN` with a `riskNote` saying so. Confirm with a text search before treating the symbol as safe to change or delete; do not proceed on the strength of a zero.
- **MUST use `query({search_query: "concept"})` for concepts/flows, `context({name: "symbolName"})` for a named symbol, or `impact` for blast radius, on read-only callers, dependencies, imports, or execution flow.** Graph first; text search only for empty/`UNKNOWN`/literals.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method before MCP/CLI impact analysis.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis, and never read `UNKNOWN` as an all-clear — it means the walk could not answer, which is the one verdict that requires confirming by other means.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit before MCP/CLI graph change analysis.

## Resources

| Resource | Use for |
| --- | --- |
| `gitnexus://repo/guidance-runtime/context` | Codebase overview, check index freshness |
| `gitnexus://repo/guidance-runtime/clusters` | All functional areas |
| `gitnexus://repo/guidance-runtime/processes` | All execution flows |
| `gitnexus://repo/guidance-runtime/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
| --- | --- |
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
