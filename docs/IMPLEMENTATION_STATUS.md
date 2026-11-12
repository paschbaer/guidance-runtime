# Implementation Status

Implemented: Rust workspace; service/CLI/tray binaries; TOML workload discovery; Windows Named Pipe and Unix development socket; JSON Lines control protocol; process start/stop/restart/status/logs; bounded restart count; tray refresh/start-all/stop-all/exit; build/install scripts and tests.

Production hardening still required: Windows Service Control Dispatcher integration; named-pipe DACL and caller-token role enforcement; Job Objects; graceful termination; durable desired state; health probes and dependency graph; event subscriptions and notifications; signed package update/rollback; runtime-service credential handling; MSI and Authenticode; Windows Service CI validation. (The updater's own artifact credential is implemented; this item refers to the runtime service.)

The source is a functional implementation baseline, not a certified production release.

## Update channels and updater

The update checker supports `stable` and `beta`, uses GitHub Releases, applies SemVer ordering, requires a SHA-256 release asset, and refuses unverified archives. Installation waits for the invoking CLI to exit, stops the target's service when it has one, creates an installation backup, replaces files, restarts the service, and restores the backup if replacement, restart or the final version-marker write fails.

### Multi-target sources

A configuration may list several targets, each with its own source, channel and installation directory. A check run reports one result per target without one target's failure suppressing the others (FR-008). The legacy flat configuration still maps to one target named `guidance-runtime`, keeps its historical wording and exit status, and stays flat when a channel is changed.

Delivered:

- The ordered target list with validation that names the offending field: duplicate or empty target name, private target without `credential_entry`, duplicate `install_dir`, a rollback directory colliding with an installation, blank or non-`.zip` `asset_pattern`, and the two configuration forms mixed in one file.
- The platform-aware archive suffix table for Windows, macOS and Linux, with unknown platforms and architectures rejected rather than silently falling back.
- The check fan-out over targets, with per-target failure isolation.
- Seven outcome states, rendered as one human-readable line per target (plus an indented detail line where the state has an actionable cause) and as a structured document via `guidance update check --format json`. The document carries `spec_version`, an RFC3339 `checked_at`, `exit_status` and one `targets[]` entry per selected target. `--target` narrows the run itself, so the document then carries exactly one element.
- Authenticated access to a private source: the credential is read from the platform's protected per-user store, attached as a bearer token only for a private target, and asset bytes come from the API asset endpoint rather than the public download address. A public target sends no `Authorization` header at all.
- A per-target version marker, `install.json` in the target's install directory, holding the target name, the installed version and the installation timestamp. A missing, unreadable, mismatched or non-semver marker resolves to `not installed` (with the transitional exception described under *Platform coverage*). The marker is written only after a successful installation, so a crash after replacement cannot claim the new version.
- Per-target installation. `guidance update install --target <name> --yes` installs one target; omitting `--target` applies every configured target that reports an available update. Only an available update is installed: up-to-date and not-installed targets are reported and skipped. A target without a `service_name` installs transactionally without service control; the runtime target keeps its historical stop → backup → replace → restart → restore sequence and its four-binary file set even when `service_name` is omitted.
- The archive determines its own content: if it contains a `bin/` directory only that directory's contents are replaced, otherwise the archive root is replaced. Replacement is copy-and-overwrite; the installation directory is never deleted, and a failed installation restores the backup and then removes the files the rejected archive had created.

The CLI starts a detached updater and exits `0` once it is running; the per-target install result and the updater's own exit status (`0` or `2`) are produced by that process after the CLI exits.

Exit status of a check run: `0` when every target produced a usable answer, `1` for a configuration or selection error detected by the updater, `2` when at least one target reported `inaccessible`, `network_error`, `invalid_artifact` or `credential_store_unavailable`. A command-line usage error (for example an invalid `--format` value) also exits `2`, because clap uses that status. The legacy flat check keeps the historical behavior of exiting `1` when the source read fails.

### Behaviour change: a refusal is reported as an access problem

A *not found*, *unauthorized* or *forbidden* answer from a source is now reported as `no access to update source`, for public targets included. Before, these arrived as a network error. This is deliberate: an unauthorized caller and a non-existent repository must be indistinguishable, or the report would disclose which private repositories exist.

The practical consequence is that a **typo in a public repository name** now reads as an access problem rather than as a connectivity problem. Check the spelling and the target's visibility before investigating the network.

### Provisioning a credential

The configuration names an entry (`credential_entry`); the value lives in the platform store under the service `guidance-update`. Provision it once per user:

| Platform | Command |
|---|---|
| Windows | Credential Manager, generic credential, target `guidance:update:<entry>` |
| macOS | `security add-generic-password -s guidance-update -a <entry> -w` |
| Linux | `secret-tool store --label='Guidance' service guidance-update username <entry>` |

When the entry is named but not provisioned, the report says so and names the entry; when the store itself cannot be consulted, the report says that instead, because the two have different remedies.

### Least privilege for the private artifact credential

The provisioned token must grant **read access to the release artifacts of the configured artifact repository only**. It must not grant access to source-code repositories and must not grant write access (FR-023). A fine-grained token limited to the artifact repository's release contents is the intended shape; a classic token with `repo` scope is broader than this feature needs. The updater cannot verify the token's scope, so the restriction is a provisioning rule, not an enforced check.

### Platform coverage

- Windows is the install platform. The transactional installer, service control, and the marker write are implemented and exercised for Windows.
- macOS and Linux can **check** for updates; the suffix table and the credential backends have a compiling arm per platform, and `ci.yml` runs the workspace on `windows-latest`, `ubuntu-latest` and `macos-latest`.
- Installation on macOS and Linux is **not implemented** and reports `installation is supported on Windows only`. It does not fall back to an unsupported path.
- The release matrix in `release.yml` remains Windows-only. macOS and Linux release legs are deferred until the private product publishes non-Windows artifacts; until then, publishing those archives has nothing to publish.

The credential backends are **not verified in CI**: no secret service daemon runs on the Linux runner and no keychain is available on macOS. Their command shape and their absent-versus-unavailable classification are unit-tested; the end-to-end behaviour against a real store is an operator acceptance step.

A machine with an installation from before the marker existed is covered by a transitional rule: no marker plus `guidance-service.exe` in the install directory means "installed at the running version". The rule applies only to `guidance-runtime` and disappears once every supported install path writes a marker.

### Known limitations

- A proxy that answers with its own authentication challenge is reported as a connectivity problem, not as an access problem, because the transport layer does not expose the distinction. On a proxied network, treat a connectivity report against a private target as possibly an access problem.
- On Linux, `secret-tool` exits 1 for both a missing item and an unreachable daemon; the backend distinguishes these only where the tool reports a distinct status.
- A Windows credential blob provisioned as UTF-8 is mangled rather than rejected.
- A not-installed target during `install` is reported and skipped with exit `0`; a *check* of the same target may report an access problem with exit `2`, because a check still tries to read the source. The two commands answer different questions.
- The global `--json` flag on `guidance` is not wired to the update subcommands; use `--format json` on `guidance update check`.
- Clap usage errors exit `2`, which collides with "at least one target failed"; configuration errors detected by the updater itself exit `1`.
