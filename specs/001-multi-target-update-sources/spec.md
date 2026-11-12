# Feature Specification: Multi-Target Update Sources

**Feature Branch**: `001-multi-target-update-sources`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "The central updater in `guidance-runtime` gets a configurable list of update targets it can fetch updates from and check whether an update exists. A second, private target `guidance` is delivered from a private artifact repository; users authenticate with their GitHub identity. An unauthorized or inaccessible source must yield a clear 'no access to update source' message."

## Clarifications

### Session 2026-11-12

- Q: Where does the machine obtain the access credential for the private artifact repository, and what happens when it is missing? → A: A named entry in the operating system's protected per-user credential store, with one entry per platform (Windows Credential Manager, macOS Keychain, Linux Secret Service); when the entry is missing the updater reports "no access to update source" and names the provisioning step. No platform is a first-class-only exception; macOS and Linux are in scope.
- Q: How does the updater determine which version of a target is currently installed? → A: From a version marker file inside the target's installation directory, written by the updater on every successful installation.
- Q: Should an update check across several targets produce a human-readable summary, a machine-readable output, or both? → A: Both: a human-readable summary as the default output plus an opt-in structured format covering every target and its outcome state.
- Q: Must mirroring a product release into the private artifact repository be a mandatory pipeline step, or may a release exist without a published artifact there? → A: Mandatory with fail-closed semantics: without an artifact in the private repository the release is not considered delivered, and the release run fails.
- Q: What backup and rollback semantics should apply to a target that has no service, such as the private product? → A: The same transactional semantics as the runtime target, minus service control: back up before replacing, restore fully on any failure, and write the version marker only on success.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Multiple Update Targets In One Configuration (Priority: P1)

An operator maintains a machine that runs `guidance-runtime` and a second product, `guidance`. Today only the runtime itself can be updated. The operator describes both products in a single configuration file and asks the updater whether an update is available for each of them; the answer names the product, the installed version and the offered version.

**Why this priority**: Without a target list there is no place to declare the private source at all. This story alone delivers value: the runtime keeps updating exactly as before while the second product becomes visible to the updater.

**Independent Test**: Configure two targets, request an update check, and confirm that each target reports its own status. A machine with only the legacy single-target configuration must behave identically to before the feature.

**Acceptance Scenarios**:

1. **Given** a configuration that names only the runtime repository in the legacy form, **When** an update check runs, **Then** the result is identical to the result before this feature existed.
2. **Given** a configuration with two targets, one public and one private artifact source, **When** an update check runs, **Then** each target reports its own status independently.
3. **Given** a configuration with two targets where the first source is unreachable, **When** an update check runs, **Then** the second target is still checked and reported.
4. **Given** a configuration that declares the same target name twice or leaves a target name empty, **When** the updater starts, **Then** it refuses to start and names the offending configuration field.

---

### User Story 2 - Private Artifact Source Access Through GitHub Identity (Priority: P1)

A user is a member of the team that is allowed to use `guidance`. Their machine can obtain updates from the private artifact repository without any additional account or password, because the GitHub identity they already have is what grants access. Users who are not in that team cannot obtain the product at all.

**Why this priority**: This is the core of the distribution decision. A private source that cannot be read is worthless, and a credential that grants source-code access to every machine would violate the stated security boundary.

**Independent Test**: With a valid identity that belongs to the permitted team, an update check against the private target reports the available version and a download completes. With an identity that does not belong to the team, the check reports the inaccessible-source message and nothing is installed.

**Acceptance Scenarios**:

1. **Given** a private target and a machine whose stored access token is valid and authorized, **When** an update check runs, **Then** the offered version is reported.
2. **Given** a private target and no stored access token, **When** an update check runs, **Then** the result is "no access to update source" naming the target, and nothing is installed.
3. **Given** a private target and a stored token that is valid but not authorized for that artifact repository, **When** an update check runs, **Then** the result is the same "no access to update source" message as for a repository that does not exist.
4. **Given** a public target, **When** an update check runs, **Then** no access token is required and the check succeeds as today.

---

### User Story 3 - Clear Outcome When Access Is Missing (Priority: P2)

An operator investigates why a machine stopped receiving `guidance` updates. The updater tells them the target could not be accessed — not that no update exists, not that the network failed — so they know to check team membership rather than connectivity.

**Why this priority**: Misdiagnosis is the main operational cost of a permission-based distribution model. It is separable from the access mechanism itself and can be delivered once the message exists.

**Independent Test**: Run an update check against a private target with an unauthorized identity and verify the reported outcome is distinguishable from "up to date", from a network failure and from an invalid artifact.

**Acceptance Scenarios**:

1. **Given** an unauthorized identity and a private target, **When** an update check runs, **Then** the outcome is reported as an access problem naming the target, distinct from "no update available".
2. **Given** an unauthorized identity and a private target, **When** the update check runs, **Then** the already installed version remains present and runnable.
3. **Given** an unreachable network, **When** an update check runs, **Then** the outcome is reported as a connectivity problem, not as an access problem.

---

### User Story 4 - Installing an Update For Each Target (Priority: P2)

An operator applies available updates. Updating `guidance` must not disturb the running runtime service, and updating the runtime must not overwrite the `guidance` installation.

**Why this priority**: Independence between targets is what makes a shared updater safe. It is only meaningful after targets can be checked, so it follows the P1 stories.

**Independent Test**: Install an update for the second target and verify that the first target's files, version and service state are unchanged.

**Acceptance Scenarios**:

1. **Given** two configured targets, **When** an update is installed for the second target, **Then** no file and no service of the first target is modified.
2. **Given** a downloaded artifact whose checksum does not match the published checksum, **When** installation is attempted, **Then** it aborts, the existing installation of that target is unchanged, and other targets are unaffected.
3. **Given** a private target with no local installation, **When** an update check runs, **Then** the target is reported as "not installed" rather than "up to date" or as an error.

---

### Edge Cases

- A target is configured as private but no credential entry name is given: this is a configuration error named on the `credential_entry` field, reported before any source is contacted. Separately, an entry that is named but not provisioned is an access problem at check time.
- The stored credential exists but has expired or been revoked: reported as an access problem, distinguishable from a connectivity problem.
- The private artifact repository is renamed or removed: reported as an access problem with the same wording as an authorization failure.
- A release for a target exists but carries no checksum artifact: the target is refused for that release, never installed unverified.
- Two targets point at the same artifact source with different channels: each target's channel restriction applies independently.
- A target's recorded installed version is absent while its installation directory exists: treated as "not installed", not as a downgrade or an error.
- The configuration file is missing or unparseable: the updater refuses all operations and names the problem; it never silently falls back to a default target list.

## Requirements *(mandatory)*

### Functional Requirements

**Target configuration**

- **FR-001**: The update configuration MUST support an ordered list of update targets, each with a stable name.
- **FR-002**: Each target MUST describe where its releases are published, whether that source is public or private, where it is installed, and how its artifact is selected for the running architecture.
- **FR-003**: The system MUST accept the existing single-target configuration unchanged and interpret it as the target named `guidance-runtime`.
- **FR-004**: The system MUST reject a configuration with a duplicate target name, an empty target name, an empty target list, or a private target without a `credential_entry`, and MUST name the offending field in the error.
- **FR-005**: For a private target, the configuration MUST name the credential entry to use and MUST NOT contain the credential value itself.

**Checking for updates**

- **FR-006**: Users MUST be able to check for an update for each configured target.
- **FR-007**: An update check result MUST identify the target, the installed version, the offered version and the channel.
- **FR-008**: A failure or missing authorization for one target MUST NOT prevent other targets from being checked; the result MUST state per target whether the check succeeded and why it did not.
- **FR-009**: A target with no local installation MUST be reported as "not installed", distinct from both "up to date" and an error.
- **FR-009a**: The installed version of a target MUST be determined from a version marker inside that target's installation directory, and the updater MUST write or update that marker on every successful installation.
- **FR-009b**: A missing or unreadable version marker MUST be reported as "not installed" rather than as "up to date", and MUST NOT be treated as a downgrade or as a corrupt configuration.
- **FR-009c**: The version marker MUST be scoped per target so that two targets installed on the same machine cannot read each other's version.

**Access to private sources**

- **FR-010**: The system MUST be able to read releases from a private artifact repository using the user's GitHub identity as the source of authorization.
- **FR-011**: The access credential MUST be obtained from the operating system's protected per-user credential store of the platform the updater runs on, and MUST NOT be stored in configuration files, in release artifacts, in logs, in command output or in error messages.
- **FR-011a**: The credential store MUST be resolved per platform rather than fixed to one platform: the native per-user secret store on Windows, on macOS and on Linux. A platform without a usable secure store MUST report that as an explicit, named condition rather than falling back to plaintext or to an unprotected file.
- **FR-011b**: The configuration names the credential entry identically across platforms; the platform-specific store is an implementation detail that MUST NOT leak into the user-facing configuration schema.
- **FR-012**: Public targets MUST remain fetchable without any credential.
- **FR-013**: When a source responds with "not found", "unauthorized" or "forbidden", or when the required credential entry is absent from the platform's credential store, the system MUST report "no access to update source" together with the target name and, for a missing entry, the provisioning step to perform.
- **FR-014**: The report for an unauthorized identity MUST be indistinguishable from the report for a non-existent repository, so the existence of the private source is not disclosed.
- **FR-015**: An access problem, an absent update, a connectivity problem and an invalid artifact MUST be reported as four distinct outcomes.
- **FR-016**: When a target cannot be accessed, its installed version MUST remain present and runnable; no removal and no partial update may occur.

**Terminology**: *target* is the canonical word for one entry in the update configuration. *Product* refers to the shipped artifact a target delivers (`guidance`, `guidance-runtime`). The two are not interchangeable: a configuration has targets, a target delivers a product.

**Fetching and installing**

- **FR-017**: Artifact retrieval for a private target MUST use the same authenticated access path as the update check and MUST NOT depend on an unauthenticated public download address.
- **FR-018**: Checksum verification against the release's checksum artifact MUST remain mandatory and fail-closed: a release without a checksum MUST NOT be installed.
- **FR-019**: A checksum mismatch MUST abort installation without modifying the existing installation of that target and without affecting other targets.
- **FR-020**: Each target MUST install into its own location and MUST NOT overwrite another target's files or control another target's service.
- **FR-020a**: A target without a service MUST follow the same transactional install semantics as the runtime target, minus service control: back up the existing installation before replacing anything, restore it fully on any failure, and only record the new version in the version marker after the replacement succeeded.
- **FR-020b**: A target whose installation failed MUST be left in its previous, runnable state and MUST report the failure; a partially replaced installation MUST NOT be reported as success or as installed at the new version.
- **FR-021**: The existing transactional installation behaviour of the `guidance-runtime` target — stop, back up, replace, restart, roll back on failure — MUST be preserved.
- **FR-022**: Channel restrictions, version ordering and architecture binding MUST apply per target and MUST NOT leak between targets.

**Least privilege**

- **FR-023**: The credential stored on a machine MUST allow reading release artifacts of the configured private source only; it MUST NOT grant access to source-code repositories and MUST NOT grant write access.

- **FR-024**: An update check across several targets MUST produce a human-readable summary as the default output, and MUST additionally offer an opt-in structured output that reports, for every configured target, its name, installed version, offered version, channel and one of the defined outcome states.
- **FR-025**: The structured output MUST expose the same outcome states as the human-readable report, so automation can distinguish an access problem from "up to date" without parsing prose.
- **FR-026**: A run in which some targets succeed and others fail MUST be distinguishable from both an all-success and an all-failure run by the run's overall result alone.

- **FR-027**: Publishing a release of the private product MUST include mirroring its artifacts into the private artifact repository as a mandatory step.
- **FR-028**: Mirroring MUST be fail-closed: when the artifacts cannot be published to the private artifact repository, the release run MUST fail and the release MUST NOT be considered delivered to end users.
- **FR-029**: A release that is visible in the product's source repository but absent from the private artifact repository MUST NOT be reported to users as an available update; the two repositories MUST NOT diverge in a way that leaves users silently without a deliverable version.

### Key Entities

- **Update Target**: A named product the updater maintains. Carries the source location, the source visibility (public or private), the credential entry name when private, the channel, the installation location, the service association when applicable, and the artifact selection rule.
- **Update Check Result**: The outcome of inspecting one target. Carries the target name and one outcome state: update available (with installed and offered version), up to date, not installed, no access to update source, connectivity failure, invalid artifact.
- **Version Marker**: A small per-target record inside the target's installation directory stating the currently installed version. Written by the updater after each successful installation; the authoritative answer to "what is installed here" and the basis for the not-installed state.
- **Credential Entry**: A named secret held in the operating system's protected storage. Referenced by name from configuration; its value is never read by configuration parsing.
- **Artifact Source**: A release publication location. Public sources need no credential; private sources require one and are subject to the undisclosed-existence rule. The private artifact source is populated only by the product's release pipeline as a mandatory mirroring step, never by manual upload.
- **Artifact Set**: The downloadable archive plus its checksum artifact for one release and one architecture, verified before installation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A machine with a pre-existing single-target configuration completes an update check with an outcome identical to the pre-feature behaviour, in 100% of cases.
- **SC-002**: With two configured targets and one source made unreachable, an update check still produces a result for both targets in a single run.
- **SC-003**: An operator can distinguish all four outcome families — access problem, no update available, connectivity problem, invalid artifact — from the updater's output alone, without reading logs.
- **SC-004**: A machine whose user is not authorized for the private source reports an access problem and installs nothing, in 0 of 0 cases producing an installed artifact.
- **SC-005**: Output for an unauthorized identity and for a non-existent source are identical, verified by comparison of the two reports.
- **SC-006**: Updating one target leaves the other target's files, version and service state unchanged, verified by inspection after the update.
- **SC-007**: No credential value appears in any produced output, verified by scanning command output, logs and error messages.
- **SC-008**: A user who is authorized completes an update of the private target without entering any credential interactively beyond what they already have on the machine.
- **SC-009**: The update path works on Windows, macOS and Linux, verified by an executed check on each platform rather than by a single-platform build; installation differences between platforms are reported explicitly instead of silently degrading.
- **SC-010**: Automation can determine the outcome state of every target from a single machine-readable run without parsing human-readable prose, verified by a script that classifies all defined outcome states.
- **SC-011**: A release whose artifacts failed to reach the private artifact repository is never observed by any user as an available update, verified by a release run that is intentionally broken and then checked end to end.

## Assumptions

- The private product's updates are published to a separate private artifact repository that holds release artifacts only, never source code; the product's source repository remains private and is not an update source for end users.
- Access to the private artifact repository is granted through GitHub team or repository permissions; the updater adds no role or allow-list of its own.
- Machines that update the private product have a GitHub identity available locally and a credential stored for it; obtaining that credential initially is an operator provisioning step outside this feature.
- The runtime continues to start its workloads from local paths; starting `guidance` and running it are out of scope, and only its update lifecycle is in scope.
- Windows, macOS and Linux are all in scope as update clients. Installation behaviour differs by platform: the transactional service-oriented flow applies to Windows, while macOS and Linux are expected to install the target's own artifacts without controlling a Windows service. A platform that cannot install a given target MUST report that explicitly and MUST still be able to check for updates.
- The existing checksum-verified, transactional installation model is reused for the runtime target and adapted, not redesigned, for the second target.
- A single update check run evaluates all configured targets; an operator wanting one target at a time is accommodated by the per-target reporting rather than by separate invocations.

## Dependencies

- The private artifact repository exists and receives mirrored release artifacts from the private product's release pipeline as a mandatory, fail-closed step.
- Team membership in GitHub is the authoritative access list; no separate authorization service is introduced.
- Each supported platform (Windows, macOS, Linux) provides a protected per-user credential store for the access token; a headless Linux host may have no Secret Service available, which is surfaced as the named unsupported-credential-store condition rather than silently degraded.

## Open Questions (for clarification or planning)

- Is the target list checkable as a whole, per target, or both?
- What format and location does the version marker take, and how is a marker written by an older updater version handled (plan decision)?
- How is an expired or revoked credential detected and communicated, as distinct from a credential that is absent?
- Does the plan choose the authenticated download path to stay on GitHub, or to allow a future move to a different host behind the same interface?
