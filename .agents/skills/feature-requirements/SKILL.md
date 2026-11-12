---
name: feature-requirements
description: Guide a structured feature-requirements session and integrate confirmed requirements into the active Spec-Kit spec.md before clarification or planning.
---

# Feature Requirements

## Purpose

Help the user define a feature's requirements in a consistent, testable, user-focused structure and record them in the active Spec-Kit `spec.md`. Do not create a separate requirements document.

Use this skill when the user wants to shape or refine requirements for one feature. Use `/speckit-specify` to create a new feature specification from a feature idea, and `/speckit-clarify` to resolve ambiguities in an already-written specification.

## User Input

```text
$ARGUMENTS
```

Use the provided feature description and all relevant conversation context. Do not ask the user to repeat information already supplied.

## Preconditions

1. From the repository root, run `.specify/scripts/powershell/check-prerequisites.ps1 -Json -PathsOnly` once and parse `FEATURE_DIR` and `FEATURE_SPEC` from its JSON output.
2. If the script fails, its output is invalid, or the active `spec.md` does not exist, do not create paths or a separate artifact. Tell the user to run `/speckit-specify` first, then retry this skill.
3. Read the active `FEATURE_SPEC`, the resolved active `spec-template`, and `.specify/memory/constitution.md` if it exists. Follow the template's section order and any applicable project principles.

## Structured Requirements Workflow

1. Review the current specification and supplied input. Retain existing confirmed requirements and identify what is missing, contradictory, vague, or not testable.
2. Organize the feature information using these requirement areas:
   - **Problem and outcome**: user need, context, and value.
   - **Users and scope**: affected user types, in-scope behavior, and explicit exclusions.
   - **User journeys**: prioritized, independently testable scenarios and why each matters.
   - **Acceptance scenarios**: observable Given/When/Then behavior for each important journey.
   - **Functional requirements**: specific, uniquely numbered, testable capabilities.
   - **Quality requirements**: relevant security and privacy, accessibility, performance, reliability, localization, or compliance expectations, expressed as verifiable outcomes.
   - **Edge cases**: boundary conditions, invalid input, and failure behavior that affect user-visible outcomes.
   - **Entities**: relevant concepts and relationships, only when the feature involves data.
   - **Success criteria**: measurable, user- or business-facing outcomes that can be verified without knowing the implementation.
   - **Assumptions and dependencies**: reasonable defaults and external or existing capabilities the feature relies on.
3. Ask concise questions only for unresolved decisions that materially change scope, user behavior, acceptance tests, data meaning, privacy, or security. Ask one question at a time, provide 2–5 meaningful choices plus a custom answer when useful, and ask no more than five questions in total. Do not ask about implementation approach, technology selection, or task breakdown. Use explicit assumptions for low-impact details when a reasonable default exists.
4. Respect requests to stop. Do not write to the specification after a stop request. If the question limit is reached, report remaining high-impact unknowns and wait for direction rather than silently deciding them.
5. Map confirmed answers and justified assumptions into the corresponding sections of `FEATURE_SPEC`, following the active template. Place quality requirements in existing sections such as Functional Requirements or Success Criteria as appropriate; do not invent new headings. Preserve section order, retain compatible existing content, and update only sections needed for the requirements. Keep story priorities and stable requirement IDs where possible; use the next unused IDs when adding requirements. Do not add implementation details or create other artifacts.
6. Review the resulting specification for testable and unambiguous functional and relevant quality requirements, measurable outcomes, consistent scope, acceptance coverage, relevant edge cases, and clearly labeled assumptions. Keep unresolved high-impact items explicit; do not present the specification as ready for planning if they remain.

## Completion Report

Report:

- The `FEATURE_SPEC` path.
- The sections added or updated.
- The number of questions asked and any unresolved high-impact items.
- Whether the requirements appear ready for `/speckit-clarify` or `/speckit-plan`.
