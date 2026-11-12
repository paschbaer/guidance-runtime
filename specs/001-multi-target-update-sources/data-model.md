# Data Model — Multi-Target Update Sources

**Feature**: `001-multi-target-update-sources`
**Phase**: 1 (Design)

Derived from `spec.md` requirements and the decisions in `research.md`. Field names shown here are
the canonical identifiers the contract and tests use; implementation types may alias them but must
not rename the wire-visible ones.

---

## UpdateTarget

A product the updater maintains.

| Field | Type | Required | Notes |
|---|---|---|---|
| `name` | string | yes | Stable identifier, unique per configuration. `guidance-runtime` is reserved for the legacy single-target form (FR-003). |
| `repository` | string | yes | `owner/repo` of the release source. |
| `visibility` | `public` \| `private` | yes | Defaults to `public` when omitted. |
| `credential_entry` | string | when `private` | **Name** of the credential store entry, never a secret (FR-005, FR-011). |
| `channel` | `stable` \| `beta` | no | Default `stable`. Applies per target (FR-022). |
| `install_dir` | path | yes | Target's own installation directory (FR-020). |
| `service_name` | string | no | Present only for targets that control a service (Windows runtime target). |
| `asset_pattern` | string | no | Optional suffix override; must end in `.zip`; empty is rejected (existing rule retained). |

**Validation rules**

- `name` non-empty, unique across the target list (FR-004).
- `targets` list non-empty; a configuration with an empty list is a configuration error (FR-004).
- `visibility = private` requires a non-empty `credential_entry`; the field MUST NOT parse a secret
  value (FR-005).
- Two targets MUST NOT share `install_dir` (prevents cross-target overwrite, FR-020).
- Legacy form: a top-level `repository`/`channel`/`install_dir`/`service_name`/`asset_pattern` with
  no `[[targets]]` parses as a single target named `guidance-runtime` (FR-003, SC-001). Loading
  must not mutate the file.

**State**

The target configuration is static for a run. Its lifecycle state is not stored on the target but
observed per run (see `TargetCheck`).

---

## TargetCheck

The outcome of inspecting one target. One instance per target per run.

| Field | Type | Notes |
|---|---|---|
| `target` | string | The `UpdateTarget.name`. |
| `state` | `TargetState` | Closed enum, see below. |
| `installed_version` | semver \| null | `null` exactly when `state = not_installed`. |
| `offered_version` | semver \| null | `null` unless `state = update_available`. |
| `channel` | `stable` \| `beta` | The channel the check used. |
| `detail` | string \| null | Human-readable cause; MUST NOT contain credential material (NFR-4). |

### TargetState

Closed enum. Every member is reachable and every member is asserted by a test.

| State | Meaning | Produced when |
|---|---|---|
| `update_available` | A permitted release newer than the installed version exists and carries a matching artifact. | Selection found a candidate (FR-006, FR-007). |
| `up_to_date` | The source was read; nothing newer is permitted on the channel. | Selection returned `None`. |
| `not_installed` | No usable version marker and no recognized legacy installation. | FR-009, FR-009a–c. |
| `inaccessible` | The source refused the read: not found, unauthorized, forbidden, or missing credential entry. | FR-013, FR-014. Deliberately does **not** distinguish non-existent from unauthorized. |
| `network_error` | The source was not reached; transport failure. | Distinguishable from `inaccessible` (FR-015). |
| `invalid_artifact` | The release exists but no permitted artifact for this platform/arch, or the checksum asset is absent. | FR-018. |
| `credential_store_unavailable` | The platform's secure store could not be consulted at all. | FR-011a. Distinct remedy from `inaccessible`. |

**Invariants**

1. `installed_version.is_none()` ⟺ `state == not_installed`.
2. `offered_version.is_some()` ⟹ `state == update_available`.
3. A run with `n` targets yields exactly `n` `TargetCheck` values, regardless of failures (FR-008).

---

## UpdateRun

The aggregate result of one check run.

| Field | Type | Notes |
|---|---|---|
| `spec_version` | integer | Schema version of the structured output; starts at `1`. |
| `checked_at` | rfc3339 | Timestamp of the run. |
| `exit_status` | integer | `0` all-clean, `1` configuration/usage error, `2` at least one target failed (R4). |
| `targets` | `TargetCheck[]` | Ordered by configured target order, stable across runs. |

---

## CredentialEntry

Not a stored entity in this feature — it is resolved from the platform store at use time.

| Aspect | Rule |
|---|---|
| Identity | `(platform_store, target.credential_entry)` |
| Value | Opaque string; never deserialized into configuration, never logged, never in `detail` (NFR-4, FR-011) |
| Absent | `state = inaccessible` with a provisioning hint (FR-013) |
| Store unusable | `state = credential_store_unavailable` (FR-011a) |
| Expired/revoked | Indistinguishable from absent at the API boundary — GitHub returns `401`/`403`, which maps to `inaccessible` (FR-014) |

---

## ArtifactSet

Derived per release per target, never persisted.

| Element | Rule |
|---|---|
| `archive` | Exactly one asset whose name matches the platform/arch suffix and the target's `asset_pattern` (FR-022, R6) |
| `checksum` | Exactly one asset whose name contains `sha256`; absence ⟹ `invalid_artifact`, never installable (FR-018) |
| `download_ref` | Authenticated asset reference, shared by archive and checksum so both use one access path (FR-017) |

---

## VersionMarker

File `install.json` inside `target.install_dir` (R3).

| Field | Type | Notes |
|---|---|---|
| `target` | string | Must equal the configured target name; a mismatch resolves to `not_installed` (FR-009c). |
| `version` | semver | The installed version. |
| `installed_at` | rfc3339 | Write timestamp, for operator diagnosis only. |

**State transitions**

```
        (no marker / unreadable / target mismatch)
                        │
                        ▼
                 not_installed ──── successful install ────► installed(version)
                        ▲                                          │
                        │                                          │
                        └──────── failed install (marker         │
                                  unchanged) ◄─────────────────────┘  next install
```

The marker is written **last** in a successful install (FR-020a). Any failure after replacement
restores the backup and leaves the previous marker untouched, so a failed install cannot claim the
new version (FR-020b).

**Transitional rule (remove after marker rollout)**: no marker **and** the target's expected
executable present in `install_dir` resolves to `installed` at the running CLI's version, so
existing runtime installations are not misreported as `not_installed`. This branch is marked for
removal and must carry a test asserting it disappears once every install path writes a marker.

---

## Relationships

```
UpdateConfig ──1:n──► UpdateTarget ──1:1──► VersionMarker        (read from install_dir)
                          │
                          └──resolves──► CredentialEntry          (private targets only)
                          │
                          └──checked──► TargetCheck ──n:1──► UpdateRun
                          │
                          └──selects──► ArtifactSet ──per release──► Download + SHA-256 verify
```

---

## Configuration schema (target list)

```toml
# Legacy form remains valid and yields one target named "guidance-runtime".
repository = "ORG/guidance-runtime"

[[targets]]
name = "guidance-runtime"
repository = "ORG/guidance-runtime"
visibility = "public"
install_dir = "C:/Program Files/Guidance Runtime"
service_name = "GuidanceRuntime"
channel = "stable"

[[targets]]
name = "guidance"
repository = "ORG/guidance-dist"
visibility = "private"
credential_entry = "guidance-update"
install_dir = "C:/Program Files/Guidance"
channel = "stable"
```

Constraints enforced at load: no duplicate `name`, no duplicate `install_dir`, non-empty `targets`
when the array is present, `credential_entry` required and non-blank for `private`.
