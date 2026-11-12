# Memory Bank

The Memory Bank contains durable project knowledge for developers and AI coding agents.
It complements source code, repository instructions, and generated code intelligence with explicit product, architecture, technology, and delivery context.

## Files

- `projectBrief.md`: Project purpose, scope, users, and success criteria.
- `productContext.md`: User needs, workflows, domain language, and business rules.
- `systemPatterns.md`: Architecture, design patterns, boundaries, and engineering conventions.
- `techContext.md`: Technology stack, tooling, environments, and operational constraints.
- `activeContext.md`: Current focus, recent decisions, open questions, and next actions.
- `progress.md`: Status of completed, active, and planned work.
- `lessonsLearned.md`: Reusable lessons obtained from incidents, mistakes, experiments, and successful approaches.
- `remaining-work-plan.md`: Tracked follow-ups from review findings and deferred work.

## Usage

Before starting substantial work:

1. Read `projectBrief.md`, `systemPatterns.md`, and `techContext.md`.
2. Read `activeContext.md` and `progress.md` for the current state.
3. Consult `productContext.md` for user-facing or domain-related changes.
4. Review `lessonsLearned.md` for relevant prior experience.

After completing substantial work:

1. Update `activeContext.md` with the new state and next actions.
2. Update `progress.md` with completed and remaining work.
3. Update architectural or technical documents when decisions change.
4. Add only reusable, evidence-based insights to `lessonsLearned.md`.

## Maintenance Rules

- Keep entries concise, factual, and current.
- Do not duplicate source code or generated documentation.
- Link to ADRs, issues, pull requests, and source files where useful.
- Mark assumptions and unresolved questions explicitly.
- Never store credentials, tokens, personal data, or other secrets here.
- Remove obsolete information instead of allowing contradictions to accumulate.
