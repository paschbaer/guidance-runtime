# CLI Contract — `guidance update` with multiple targets

**Feature**: `001-multi-target-update-sources`
**Status**: Design (Phase 1)

This is the externally observed contract. Internal function signatures may change; the exit codes,
human text families and JSON schema below are the surface the constitution calls a public contract.

---

## 1. Commands

### `guidance update check [--config <path>] [--format text|json] [--target <name>]`

Checks **every configured target** by default. `--target` narrows the run to one named target and
is an operator convenience, not a separate mode: the same code path and the same output shape apply
(one element in `targets`).

- `--format text` (default): human-readable summary, one line per target.
- `--format json`: one JSON document per run (schema below).

### `guidance update install [--config <path>] [--target <name>] [--yes]`

Installs the available update for the named target, or for every target that reports
`update_available` when `--target` is omitted. `--yes` remains mandatory for any mutation.

### `guidance update channel [<stable|beta>] [--config <path>] [--target <name>]`

Reads or sets the channel for one target. Without `--target`, a multi-target configuration is
refused with a message naming the configured targets — silently applying a channel to all targets
would hide which product changed.

---

## 2. Exit status

| Code | Meaning | Condition |
|---|---|---|
| `0` | Clean | Every target reported `up_to_date`, or `update_available` in a check run. |
| `1` | Usage / configuration error | Unparseable config, duplicate target name, missing `credential_entry` for a private target, unknown `--target`, install without `--yes`. Nothing was checked or installed. |
| `2` | At least one target failed | Any target reported `inaccessible`, `network_error`, `invalid_artifact` or `credential_store_unavailable`, while the run still completed for the others. |

The three-way split exists so automation can distinguish "nothing to do" from "something is wrong
with one product" without parsing prose (SC-003, SC-010).

---

## 3. Human-readable output

One line per target, target name first, stable order matching configuration order.

```
guidance-runtime  up to date (1.3.4, stable)
guidance          update available: 2.0.1 -> 2.0.3 (stable)
guidance          not installed
guidance          no access to update source
guidance          cannot reach update source (network error: connection timed out)
guidance          release has no usable artifact for this platform
guidance          credential store unavailable on this system
```

**Required properties**

- The phrase **`no access to update source`** appears verbatim for `inaccessible` (FR-013,
  SC-005). It is identical whether the repository is absent or the identity is unauthorized.
- `network_error` and `invalid_artifact` never reuse the access phrase (FR-015).
- No credential value, token fragment or store path appears in any line (NFR-4).
- For a private target with an absent entry, the line carries a provisioning hint on a second,
  indented line (`provision it with: guidance update credentials --target <name>`).

---

## 4. Structured output (`--format json`)

```json
{
  "spec_version": 1,
  "checked_at": "2026-11-12T09:14:03Z",
  "exit_status": 2,
  "targets": [
    {
      "target": "guidance-runtime",
      "state": "up_to_date",
      "installed_version": "1.3.4",
      "offered_version": null,
      "channel": "stable",
      "detail": null
    },
    {
      "target": "guidance",
      "state": "inaccessible",
      "installed_version": "2.0.1",
      "offered_version": null,
      "channel": "stable",
      "detail": "no access to update source"
    }
  ]
}
```

**Schema rules**

- `state` is one of the closed set: `update_available`, `up_to_date`, `not_installed`,
  `inaccessible`, `network_error`, `invalid_artifact`, `credential_store_unavailable`.
  Adding a member is a schema change and bumps `spec_version`.
- `targets` length equals the number of configured (or selected) targets, always (FR-008).
- `detail` MUST NOT contain credential material. Tests assert the absence of the token value.
- Field order is stable; the document is emitted once, not streamed.

---

## 5. Configuration contract

```toml
# Legacy single-target form — unchanged, yields target "guidance-runtime".
repository = "ORG/guidance-runtime"
channel = "stable"
install_dir = "C:/Program Files/Guidance Runtime"
service_name = "GuidanceRuntime"

# Multi-target form.
[[targets]]
name = "guidance-runtime"
repository = "ORG/guidance-runtime"
visibility = "public"
channel = "stable"
install_dir = "C:/Program Files/Guidance Runtime"
service_name = "GuidanceRuntime"

[[targets]]
name = "guidance"
repository = "ORG/guidance-dist"
visibility = "private"
credential_entry = "guidance-update"
channel = "stable"
install_dir = "C:/Program Files/Guidance"
```

**Load-time errors** (all exit status `1`, message names the offending field):

| Condition | Message subject |
|---|---|
| `targets` present but empty | `targets` |
| Duplicate `name` | the duplicated name |
| Empty `name` | `name` |
| `visibility = "private"` without `credential_entry` | `credential_entry` |
| Two targets with the same `install_dir` | `install_dir` |
| `asset_pattern` not ending in `.zip` or blank | `asset_pattern` |
| Both legacy keys and `targets` present | explicit refusal (ambiguous intent) |

---

## 6. Credential provisioning contract (out of scope to implement, in scope to document)

The updater never writes credentials. The operator provisions the entry once; the updater only
reads it.

| Platform | Store | Entry lookup |
|---|---|---|
| Windows | Credential Manager (generic) | service name `guidance:update:<entry>` |
| macOS | Keychain (generic password) | `-s guidance-update -a <entry>` |
| Linux | Secret Service (D-Bus) | `service=guidance-update`, `username=<entry>` |

The configuration **names** the entry (for example `guidance-update`); it never carries the value.
A missing entry yields the `no access to update source` outcome with a provisioning hint.

---

## 7. Release-side contract (consumed, produced elsewhere)

Recorded here because the updater depends on it, not because this repository implements it.

| Element | Rule |
|---|---|
| Archive assets | `guidance-<version>-<platform>-<arch>.zip` per published platform |
| Checksum asset | One asset per release whose name contains `sha256`, listing the archive digest |
| Mirroring | Publish to the private artifact repository is a mandatory, fail-closed pipeline step (FR-027–FR-029) |
| Access | Consumers read via authenticated GitHub API; `browser_download_url` is never used for private sources |
