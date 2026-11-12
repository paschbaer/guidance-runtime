---
name: guidance-setup
description: Creates a fresh .guidance/ configuration for this project via the Guidance configuration assistant, driven by a target-repo template (generic, rust, nodejs, csharp). Use when the user asks to "set up guidance", "create the guidance config", "guidance setup", or wants a new configuration generated from their own answers. Asks every wizard question one at a time and waits for the operator's answer — never answers on their behalf and never assumes defaults.
---

# Guidance Setup — Template-Based Configuration

## When to Use

- The project has no `.guidance/` configuration yet, or a new configuration is explicitly requested.
- The wizard opens with ONE question: `template` (target-repo template: generic / rust / nodejs / c#). It selects BOTH the wizard questions and the generated guidance files.

## Never

- Do NOT answer wizard questions on the operator's behalf.
- Do NOT assume default values without explicit confirmation.
- Do NOT batch all questions into one form — one question at a time, then WAIT.

## Procedure

1. Run the `setup_guidance_start` tool (pass the repository name as `workspaceNameHint`).
2. Present the `template` question (options with labels/descriptions come from the runtime template listing) and WAIT for the operator's choice.
3. Answer the `template` question via `setup_guidance_answer` — the rest of the catalog is rendered from the chosen template's `questions.json`.
4. Take the current question, present it with its options, help text, and a derived recommendation, and ask the operator — **one question at a time**.
5. WAIT for the operator's answer before continuing; feed each answer via `setup_guidance_answer` (accumulating ALL previous answers) and repeat until the wizard reports `done`.
6. Run `setup_guidance_generate` with the collected answers.
7. Write the returned files to the project root (`.guidance/` structure, including schemas and scripts).
8. Report the created files and the chosen configuration (including the selected template) to the operator.

## Notes

- The old `configSource` (adopt/fresh) modes no longer exist; external reference adoption was removed.
- Template edits under the templates root take effect WITHOUT redeploying guidance — the files are read at generation time (`GUIDANCE_TEMPLATES_DIR` points at a runtime-mounted templates root in deployed setups).
