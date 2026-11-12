---
name: Fill AGENTS.md
description: "Customizes AGENTS.md with a short repository description, the programming language, and basic implementation rules."
argument-hint: "Optional: what is already known (repo purpose, language, coding standards)?"
---

Customize `AGENTS.md` in this repository with project-specific content: a short repository description, the programming language used, and basic implementation rules.

## Content to Establish

1. **Short repository description** — two to three sentences: what the project is and what problem it solves.
2. **Programming language** — the language(s) actually used, including version, standard, or style guide where relevant.
3. **Basic implementation rules** — the project's ground rules for writing code. Examples: use dependency injection, define interfaces for external dependencies, write unit tests for new behavior, error-handling conventions, naming conventions. Keep the list concrete and enforceable, not aspirational.

## Rules

- Base every statement on verifiable facts: the repository itself (dependency manifests, source files, configuration) or explicit answers from the user. Label assumptions as assumptions.
- Never invent standards, tools, or conventions. If information is missing, ask a concrete question; if the user cannot answer, keep a clearly marked open question instead of fabricating content.
- Preserve all existing rules in `AGENTS.md`. Extend and refine, never weaken or delete. Contradictions between new and existing rules must be resolved with the user before writing.
- Follow the backup rule in `AGENTS.md` (rule updates): back up the file to `AGENTS.md.bak` before the first modification.
- Never write credentials, tokens, personal data, or secrets into `AGENTS.md`.
- Do not modify files other than `AGENTS.md` (the backup file and the Memory Bank updates required by AGENTS.md §5 excepted).

## Procedure

1. **Gather context:** Read `AGENTS.md` completely and inspect the repository (README, dependency manifests, source layout, test setup).
2. **Interview the user** for anything the repository cannot answer. Ask focused questions, at most three at a time, in this order:
   1. Repository purpose and short description
   2. Programming language(s), version, and style guide
   3. Basic implementation rules (e.g., dependency injection, interfaces, unit tests)
3. **Integrate the content**, adapting to the existing structure of `AGENTS.md`:
   - Create a `## Project Context` section after the main heading for description and language.
   - Extend the `## Architecture` section (or create a dedicated `## Implementation Rules` section) with the ground rules as a bulleted list.
   - Derive wording and terminology from the actual codebase.
4. **Report:** Show the added sections and the diff summary; point out any contradiction found between new and existing rules, and any remaining open questions.

## Acceptance Criteria

- `AGENTS.md` contains a short repository description, the programming language, and a concrete list of implementation rules.
- All rules from the original `AGENTS.md` are preserved; a backup exists at `AGENTS.md.bak`.
- No fabricated content; unresolved items are explicit, marked open questions.
- No secrets or personal data were added.
