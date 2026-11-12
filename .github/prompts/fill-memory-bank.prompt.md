---
name: Fill Memory Bank
description: "Replaces the TODO placeholders in memory-bank/ with repository-specific content through a guided interview."
argument-hint: "Optional: what is already known about the project (name, purpose, stack)?"
---

Fill the Memory Bank in `memory-bank/` with repository-specific content by replacing all TODO placeholders.

## Rules

- Base every statement on verifiable facts: the repository itself, its README, configuration files, or explicit answers from the user. Label assumptions as assumptions.
- Never invent stakeholders, commands, metrics, or decisions. If information is missing, ask a concrete question; if the user cannot answer, keep a clearly marked open question instead of fabricating content.
- Never write credentials, tokens, personal data, or secrets into any Memory Bank file.
- Keep entries concise and factual; follow the structure and maintenance rules in `memory-bank/README.md`.
- Remove the template banner at the top of each file once its placeholders are resolved.
- Do not modify files outside `memory-bank/`.

## Procedure

1. **Gather context:** Inspect the repository (README, dependency manifests, build configuration, directory layout) and read all files in `memory-bank/`.
2. **Interview the user** for anything the repository cannot answer. Ask focused questions, at most three at a time, in this order:
   1. Purpose, target users, and success criteria (`projectBrief.md`)
   2. User needs, workflows, and business rules (`productContext.md`)
   3. Architecture, boundaries, and patterns (`systemPatterns.md`)
   4. Toolchain, commands, and environments (`techContext.md`)
3. **Fill the files** in the same order. For each file:
   - Replace TODO placeholders with the gathered content.
   - Update `Last Updated` (date and author).
   - Remove the template banner if no unresolved placeholder remains in that file.
4. **Update `activeContext.md` and `progress.md`:** set current focus, next actions, initial milestones, and record that the Memory Bank was filled.
5. **Report:** List per file what was filled, which placeholders remain open, and which open questions need user input.

## Acceptance Criteria

- No fabricated content; every unresolved item is an explicit, marked open question.
- All Memory Bank files contain no unresolved TODO placeholders.
- No secrets or personal data were added.
