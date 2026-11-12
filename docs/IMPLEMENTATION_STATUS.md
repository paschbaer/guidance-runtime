# Implementation Status

Implemented: Rust workspace; service/CLI/tray binaries; TOML workload discovery; Windows Named Pipe and Unix development socket; JSON Lines control protocol; process start/stop/restart/status/logs; bounded restart count; tray refresh/start-all/stop-all/exit; build/install scripts and tests.

Production hardening still required: Windows Service Control Dispatcher integration; named-pipe DACL and caller-token role enforcement; Job Objects; graceful termination; durable desired state; health probes and dependency graph; event subscriptions and notifications; signed package update/rollback; Credential Manager; MSI and Authenticode; Windows CI validation.

The source is a functional implementation baseline, not a certified production release.

## Update channels and updater

The update checker supports `stable` and `beta`, uses GitHub Releases, applies SemVer ordering, requires a SHA-256 release asset, and refuses unverified archives. Installation uses a temporary copy of `guidance-updater.exe`, waits for the invoking CLI to exit, stops the service, creates an installation backup, replaces binaries, restarts the service, and restores the backup if replacement or restart fails.

### Multi-target sources (in progress)

A configuration may list several targets, each with its own source, channel and
installation directory, and a check run reports one result per target without
one target's failure suppressing the others. Only the runtime target reports an
installed version; every other target currently reports `not installed`.

Delivered so far: the target list and its validation, the platform-aware
archive suffix table (Windows, macOS, Linux), the fan-out over targets, the
seven outcome states, and the legacy flat configuration keeping its historical
output contract. The flat form stays flat when a channel is changed.

Authenticated access to a private source is implemented: the credential is read
from the platform's protected per-user store, attached as a bearer token only
for a private target, and asset bytes come from the API asset endpoint rather
than the public download address. A public target sends no `Authorization`
header at all.

### Behaviour change: a refusal is reported as an access problem

A *not found*, *unauthorized* or *forbidden* answer from a source is now
reported as `no access to update source`, for public targets included. Before,
these arrived as a network error. This is deliberate: an unauthorized caller and
a non-existent repository must be indistinguishable, or the report would
disclose which private repositories exist.

The practical consequence is that a **typo in a public repository name** now
reads as an access problem rather than as a connectivity problem. Check the
spelling and the target's visibility before investigating the network.

### Provisioning a credential

The configuration names an entry (`credential_entry`); the value lives in the
platform store under the service `guidance-update`. Provision it once per user:

| Platform | Command |
|---|---|
| Windows | Credential Manager, generic credential, target `guidance-update:<entry>` |
| macOS | `security add-generic-password -s guidance-update -a <entry> -w` |
| Linux | `secret-tool store --label='Guidance' service guidance-update username <entry>` |

When the entry is named but not provisioned, the report says so and names the
entry; when the store itself cannot be consulted, the report says that instead,
because the two have different remedies.

The credential backends are **not verified in CI**: no secret service daemon
runs on the Linux runner and no keychain is available on macOS. Their command
shape and their absent-versus-unavailable classification are unit-tested; the
end-to-end behaviour against a real store is an operator acceptance step.

Not yet delivered, and therefore not usable:

- A proxy that answers with its own authentication challenge is reported as a
  connectivity problem, not as an access problem, because the transport layer
  does not expose the distinction. On a proxied network, treat a connectivity
  report against a private target as possibly an access problem.
- Structured (`json`) output. The `--format` flag documented in the feature
  contract is not present yet; only the human-readable form exists.
- Per-target installation: installing a target still reaches the runtime's
  service name unless the target overrides `service_name`, and a service-less
  archive cannot be installed.
- A macOS CI leg. The macOS suffix mapping exists and is unit-tested on every
  host, but no CI leg runs on macOS, and the release matrix publishes Windows
  archives only.
- Operator documentation of the multi-target form beyond
  `config/updates.example.toml`.
