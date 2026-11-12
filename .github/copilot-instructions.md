# Copilot Custom Instructions

These instructions apply to every GitHub Copilot interaction in this repository. They complement `AGENTS.md`, which remains the authoritative rule set. On conflicts, `AGENTS.md` wins.

## General Behavior

- Follow `AGENTS.md` before writing or modifying any file.
- Make minimal, focused changes; keep the style of the surrounding code.
- Do not perform corrective actions beyond the given task without asking first ("Should I do X?"). Ambiguity resolves to not acting.
- Never commit, push, delete files, or modify rule files unless explicitly requested.

## Repository Knowledge

- Read the Memory Bank in `memory-bank/` before starting substantial work:
  `projectBrief.md`, `systemPatterns.md`, `techContext.md`, `activeContext.md`, `progress.md`.
- Update `activeContext.md` and `progress.md` after substantial work, as described in `memory-bank/README.md`.
- Consult `lessonsLearned.md` to avoid repeating known mistakes.

## Code Quality

- Fix root causes instead of symptoms; avoid unnecessary complexity.
- Keep build, test, lint, and run commands in sync with `memory-bank/techContext.md`.
- Add or adjust tests when changing behavior, and state which validation was actually executed.
- Never add credentials, tokens, or secrets to source files or documentation.

## Communication

- Be concise and direct; answer in the language the user writes in.
- Label inferences as such and state what could not be verified.
- For bug fixes, follow the Bugfix Protocol in `AGENTS.md` (reproduction, root cause, impact, fix strategy, verification plan).
