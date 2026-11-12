---
name: update-memory-bank
description: Updates the Memory Bank in memory-bank/ after substantial work. Use when a task changed architecture, tooling, project focus, progress, or produced a reusable lesson.
---

# Update Memory Bank

## When to Use

- After completing substantial work (features, refactoring, bug fixes, configuration changes).
- When an architectural or technological decision was made or changed.
- After resolving a difficult or recurring bug.
- At the end of a session that moved the project forward.

Do not use it for trivial edits or unchanged placeholders.

## Procedure

1. Read `memory-bank/README.md` for the file responsibilities and maintenance rules.
2. Assess what actually changed, then update only the affected files:

| Change | File to update |
|---|---|
| Current focus, decisions, open questions, next actions | `activeContext.md` |
| Completed/remaining work, milestones, change log | `progress.md` |
| Architecture, patterns, boundaries | `systemPatterns.md` |
| Toolchain, commands, environments | `techContext.md` |
| Product scope, users, domain rules | `projectBrief.md` / `productContext.md` |
| Reusable insight from a bug, mistake, or experiment | `lessonsLearned.md` |

3. Apply the maintenance rules:
   - Keep entries concise, factual, and current.
   - Remove obsolete information instead of leaving contradictions.
   - Mark assumptions and unresolved questions explicitly.
   - Never store credentials, tokens, personal data, or secrets.
4. Remove TODO placeholders that the work has resolved.
5. Report in chat which Memory Bank files were updated and why.

## Output

A short summary listing each updated file with a one-line rationale.
