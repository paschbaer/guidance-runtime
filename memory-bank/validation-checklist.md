# Manual Environment Validation Checklist

Executed record for the `progress.md` Planned item "Validate the development
environment end to end (install/uninstall via `scripts/*.ps1`)". Run on a real
Windows machine with admin rights; record the date, the machine name and every
exit code. Nothing on this list may be marked done from memory — paste the
observed output or name the exit code for each step.

## Prerequisites

- [ ] `cargo build --workspace --release` succeeds (the binaries the scripts install)
- [ ] Repo root is `D:\repos\guidance-runtime`; scripts are `scripts/install-dev.ps1`,
      `scripts/uninstall-dev.ps1`
- [ ] No prior `GuidanceRuntime` service on the machine
      (`sc.exe query GuidanceRuntime` → `FAILED 1060`)

## Install

- [ ] `scripts/install-dev.ps1` (admin) → exit `0`; paste the final output lines
- [ ] `sc.exe query GuidanceRuntime` → `RUNNING`
- [ ] `guidance status` → exit `0`, runtime summary via IPC (service reachable)
- [ ] `guidance list` → exit `0`, enabled workloads listed as configured
- [ ] `guidance start <id>` / `guidance stop <id>` → exit `0` each; process visible in Task
      Manager while started
- [ ] `guidance logs <id> --tail 5` → exit `0`, five lines
- [ ] Tray icon present; menu entries respond (manual observation, no exit code)

## Uninstall

- [ ] `scripts/uninstall-dev.ps1` (admin) → exit `0`; paste the final output lines
- [ ] `sc.exe query GuidanceRuntime` → `FAILED 1060`
- [ ] `guidance status` now fails to reach the IPC endpoint (connection error, non-zero exit)

## Update path (optional but recommended while installed)

- [ ] `guidance update check` against the configured source → exit `0`/`2` per
      `update-cli.md` §2 (record which)
- [ ] `guidance update install` **not** executed unless a real update is intended — if it is,
      record the updater's own exit status (`0` or `2`) separately

## Recording

Record in `memory-bank/progress.md` (Change Log entry) and tick the Planned
item, pasting one line per step: date, machine, step, exit code. Any failing
step is a Bugfix-Protocol finding, not a note.
