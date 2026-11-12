---
name: guidance-setup-adopt
description: Adopts the proven built-in reference .guidance/ configuration for this repo via the Guidance configuration assistant (adopt mode with builtin reference). Use when the user says "guidance setup adopt", wants the reference/template configuration instead of a fresh one, or mentions adopting the builtin guidance config. Only the remaining questions are asked — one at a time, waiting for the operator's answer.
---

# Guidance Setup — Adopt Reference Configuration

## When to Use

- This repo should work with Guidance and the operator wants the proven reference configuration (adopt mode) instead of a fresh one.
- `insight`, `gitnexus`, and `gates` are derived from the reference template and will not be asked.

## Never

- Do NOT answer wizard questions on the operator's behalf.
- Do NOT assume default values without explicit confirmation.
- Do NOT batch all questions into one form — one question at a time, then WAIT.

## Procedure

1. Run the `setup_guidance_start` tool (pass the repository name as `workspaceNameHint`).
2. Answer `configSource` with `adopt` and leave `referencePath` empty (or set it to `builtin`).
3. Ask the REMAINING questions **one at a time**, present options and a derived recommendation, and WAIT for the operator's answer before continuing; feed each answer via `setup_guidance_answer` until the wizard reports `done`.
4. Run `setup_guidance_generate` with the collected answers.
5. Write the returned files to the project root (`.guidance/` structure, including schemas and scripts).
6. Add `.guidance/state/` to the `.gitignore` (append if the entry is missing).
7. Remind the operator to run `gitnexus analyze --no-stats` in this repository.
8. Report the created files and the chosen configuration to the operator.
