//! Behavioural tests for the update pipeline, driven entirely through the
//! ports (no network, no filesystem outside temp dirs).

use crate::{
    check_with, download_and_verify_with, is_allowed, parse_checksum, select, Access, Asset,
    AuthenticatedAsset, Channel, CredentialStore, CredentialStoreError, Downloader, Release,
    Releases, Secret, UpdateTarget,
};
use anyhow::Result;
use semver::Version;

/// Returns a fixed secret, so a private-target path can be exercised without a
/// real credential store.
struct FixedCredentialStore(Option<Secret>);

impl CredentialStore for FixedCredentialStore {
    fn get<'a>(
        &'a self,
        _entry: &'a str,
    ) -> crate::BoxFuture<'a, std::result::Result<Option<Secret>, CredentialStoreError>> {
        let value = self.0.clone();
        Box::pin(async move { Ok(value) })
    }
}

/// Panics if consulted, proving that a path which must not read credentials
/// really does not.
struct NoCredentialStore;

impl CredentialStore for NoCredentialStore {
    fn get<'a>(
        &'a self,
        _entry: &'a str,
    ) -> crate::BoxFuture<'a, std::result::Result<Option<Secret>, CredentialStoreError>> {
        Box::pin(async move { panic!("this path must not consult the credential store") })
    }
}

/// Reports the store as unusable, which must be distinguishable from an absent
/// entry.
struct UnavailableCredentialStore;

impl CredentialStore for UnavailableCredentialStore {
    fn get<'a>(
        &'a self,
        _entry: &'a str,
    ) -> crate::BoxFuture<'a, std::result::Result<Option<Secret>, CredentialStoreError>> {
        Box::pin(async move {
            Err(CredentialStoreError::Unavailable(
                "no secret service on this system".into(),
            ))
        })
    }
}

struct FixedReleases(Vec<Release>);

impl Releases for FixedReleases {
    fn list<'a>(&'a self, _access: Access<'a>) -> crate::BoxFuture<'a, Result<Vec<Release>>> {
        Box::pin(async move { Ok(self.0.clone()) })
    }
}

/// Fails for one repository and succeeds for the other, so a test can drive
/// the source-error branch for exactly one target.
struct FailingFor(&'static str, Vec<Release>);

impl Releases for FailingFor {
    fn list<'a>(&'a self, access: Access<'a>) -> crate::BoxFuture<'a, Result<Vec<Release>>> {
        let fails = access.repository == self.0;
        let repository = access.repository.to_string();
        let releases = self.1.clone();
        Box::pin(async move {
            if fails {
                anyhow::bail!("simulated source failure for {repository}")
            }
            Ok(releases)
        })
    }
}

/// Records the bearer token it was given, so a test can assert whether a
/// credential reached the source and whether a public target sent none.
struct RecordingReleases {
    releases: Vec<Release>,
    seen: std::sync::Arc<std::sync::Mutex<Vec<Option<String>>>>,
}

impl RecordingReleases {
    fn new(
        releases: Vec<Release>,
    ) -> (Self, std::sync::Arc<std::sync::Mutex<Vec<Option<String>>>>) {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        (
            Self {
                releases,
                seen: seen.clone(),
            },
            seen,
        )
    }
}

impl Releases for RecordingReleases {
    fn list<'a>(&'a self, access: Access<'a>) -> crate::BoxFuture<'a, Result<Vec<Release>>> {
        self.seen
            .lock()
            .expect("recording lock")
            .push(access.token.map(|t| t.expose().to_string()));
        let releases = self.releases.clone();
        Box::pin(async move { Ok(releases) })
    }
}

struct FixedDownloader(Vec<u8>, Vec<u8>);

impl Downloader for FixedDownloader {
    fn get<'a>(&'a self, asset: &'a AuthenticatedAsset) -> crate::BoxFuture<'a, Result<Vec<u8>>> {
        // The archive and the checksum now share the same endpoint shape, so the
        // double distinguishes them by the asset id: the release fixture numbers
        // the archive before the checksum.
        let bytes = if asset.url.ends_with("1001") {
            self.1.clone()
        } else {
            self.0.clone()
        };
        Box::pin(async move { Ok(bytes) })
    }
}

fn release(tag: &str, prerelease: bool, assets: &[&str]) -> Release {
    Release {
        tag_name: tag.into(),
        prerelease,
        draft: false,
        assets: assets
            .iter()
            .enumerate()
            .map(|(i, name)| Asset {
                name: (*name).into(),
                id: 1000 + i as u64,
            })
            .collect(),
    }
}

fn config(channel: Channel) -> UpdateTarget {
    UpdateTarget {
        name: "guidance-runtime".into(),
        repository: "owner/repo".into(),
        visibility: crate::Visibility::Public,
        credential_entry: None,
        channel,
        install_dir: "C:/tmp/install".into(),
        service_name: Some("GuidanceRuntime".into()),
        asset_pattern: Some("windows-x64.zip".into()),
    }
}

fn v(s: &str) -> Version {
    Version::parse(s).unwrap()
}

const ASSET: &str = "windows-x64.zip";
const X64: &str = "windows-x64.zip";
const ARM64: &str = "windows-arm64.zip";

#[test]
fn beta_channel_accepts_a_plain_release() {
    let r = release("v1.2.0", false, &[ASSET]);
    assert!(is_allowed(Channel::Beta, &r, &v("1.2.0")));
}

#[test]
fn beta_channel_accepts_a_beta_prerelease() {
    let r = release("v1.2.0-beta.1", true, &[ASSET]);
    assert!(is_allowed(Channel::Beta, &r, &v("1.2.0-beta.1")));
}

#[test]
fn beta_channel_rejects_other_prerelease_labels() {
    let r = release("v1.2.0-alpha.1", true, &[ASSET]);
    assert!(!is_allowed(Channel::Beta, &r, &v("1.2.0-alpha.1")));
}

#[test]
fn beta_channel_rejects_a_flagged_prerelease_with_empty_pre() {
    // GitHub marks the release as prerelease, semver carries no pre-release
    // part: this is neither a published stable nor a beta build.
    let r = release("v1.2.0", true, &[ASSET]);
    assert!(!is_allowed(Channel::Beta, &r, &v("1.2.0")));
}

#[test]
fn stable_channel_accepts_only_plain_releases() {
    let plain = release("v1.2.0", false, &[ASSET]);
    let flagged = release("v1.2.0", true, &[ASSET]);
    let beta = release("v1.2.0-beta.1", true, &[ASSET]);
    assert!(is_allowed(Channel::Stable, &plain, &v("1.2.0")));
    assert!(!is_allowed(Channel::Stable, &flagged, &v("1.2.0")));
    assert!(!is_allowed(Channel::Stable, &beta, &v("1.2.0-beta.1")));
}

#[test]
fn selection_picks_the_highest_allowed_version() {
    let releases = vec![
        release("v1.1.0", false, &[ASSET]),
        release("v1.3.0", false, &[ASSET]),
        release("v1.2.0", false, &[ASSET]),
    ];
    let picked = select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        ASSET,
        ASSET,
        releases,
    )
    .unwrap();
    assert_eq!(picked.version, v("1.3.0"));
}

#[test]
fn selection_ignores_versions_not_above_current() {
    let releases = vec![release("v1.0.0", false, &[ASSET])];
    assert!(select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        ASSET,
        ASSET,
        releases
    )
    .is_none());
}

#[test]
fn selection_requires_a_matching_asset() {
    let releases = vec![release("v2.0.0", false, &["linux-x64.zip"])];
    assert!(select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        ASSET,
        ASSET,
        releases
    )
    .is_none());
}

#[test]
fn selection_attaches_the_checksum_url_when_present() {
    let releases = vec![release("v2.0.0", false, &[ASSET, "windows-x64.zip.sha256"])];
    let picked = select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        ASSET,
        ASSET,
        releases,
    )
    .unwrap();
    // The checksum resolves to the authenticated API endpoint, exactly like the
    // archive. A public download address would answer 404 for a private source,
    // so the two must never diverge into one authenticated and one public
    // request.
    assert_eq!(
        picked.checksum_url.as_deref(),
        Some("https://api.github.com/repos/owner/repo/releases/assets/1001")
    );
}

/// The archive uses the same authenticated endpoint, never a public download
/// address: `Asset` no longer even carries one, so this pins the shape rather
/// than a choice.
#[test]
fn selection_resolves_the_archive_to_the_authenticated_endpoint() {
    let releases = vec![release("v2.0.0", false, &[ASSET])];
    let picked = select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        ASSET,
        ASSET,
        releases,
    )
    .unwrap();
    assert_eq!(
        picked.download_url,
        "https://api.github.com/repos/owner/repo/releases/assets/1000"
    );
    assert!(
        !picked.download_url.contains("example.test"),
        "no public download address may be used"
    );
}

#[test]
fn checksum_parsing_prefers_the_asset_line() {
    let text = "aaaa  other.zip\nbbbb  windows-x64.zip\n";
    assert_eq!(parse_checksum(text, ASSET).as_deref(), Some("bbbb"));
}

#[test]
fn checksum_parsing_falls_back_to_the_first_line() {
    let text = "cccc\n";
    assert_eq!(parse_checksum(text, ASSET).as_deref(), Some("cccc"));
}

#[test]
fn checksum_parsing_lowercases() {
    assert_eq!(parse_checksum("DDDD", ASSET).as_deref(), Some("dddd"));
}

#[test]
fn checksum_parsing_of_empty_text_is_none() {
    assert!(parse_checksum("", ASSET).is_none());
}

async fn verified(payload: &[u8], checksum_text: &str) -> Result<Vec<u8>> {
    let source = FixedReleases(vec![release(
        "v2.0.0",
        false,
        &[ASSET, "windows-x64.zip.sha256"],
    )]);
    let update = check_with(&source, &config(Channel::Stable), None, &v("1.0.0"))
        .await?
        .unwrap();
    let downloader = FixedDownloader(payload.to_vec(), checksum_text.as_bytes().to_vec());
    download_and_verify_with(&downloader, &update, None).await
}

#[tokio::test]
async fn download_succeeds_with_a_matching_checksum() {
    let payload = b"payload";
    let digest = crate::params::sha256_hex(payload);
    assert_eq!(verified(payload, &digest).await.unwrap(), payload);
}

#[tokio::test]
async fn download_fails_on_a_mismatching_checksum() {
    let err = verified(b"payload", &"0".repeat(64)).await.unwrap_err();
    assert!(err.to_string().contains("SHA-256 mismatch"), "{err}");
}

#[tokio::test]
async fn download_refuses_an_update_without_checksum_asset() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[ASSET])]);
    let update = check_with(&source, &config(Channel::Stable), None, &v("1.0.0"))
        .await
        .unwrap()
        .unwrap();
    let downloader = FixedDownloader(b"payload".to_vec(), Vec::new());
    let err = download_and_verify_with(&downloader, &update, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("no SHA-256 asset"), "{err}");
}

#[tokio::test]
async fn check_with_returns_the_newest_allowed_release() {
    let source = FixedReleases(vec![
        release("v1.5.0", false, &[ASSET]),
        release("v1.9.0-beta.1", true, &[ASSET]),
        release("v1.7.0", false, &[ASSET]),
    ]);
    let stable = check_with(&source, &config(Channel::Stable), None, &v("1.0.0"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stable.version, v("1.7.0"));

    let beta = check_with(&source, &config(Channel::Beta), None, &v("1.0.0"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(beta.version, v("1.9.0-beta.1"));
}

#[test]
fn x64_build_does_not_select_the_arm64_asset() {
    let releases = vec![release("v2.0.0", false, &["windows-arm64.zip"])];
    assert!(select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        X64,
        X64,
        releases
    )
    .is_none());
}

#[test]
fn arm64_build_does_not_select_the_x64_asset() {
    let releases = vec![release("v2.0.0", false, &["windows-x64.zip"])];
    assert!(select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        ARM64,
        ARM64,
        releases
    )
    .is_none());
}

#[test]
fn a_loose_fragment_still_cannot_cross_architectures() {
    // "64.zip" is a suffix of BOTH architectures. Order the candidates so the
    // FOREIGN one comes first: the guard, not list order, must reject it.
    let arm_first = vec![release(
        "v2.0.0",
        false,
        &["windows-arm64.zip", "windows-x64.zip"],
    )];
    let picked = select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        "64.zip",
        X64,
        arm_first,
    )
    .unwrap();
    assert_eq!(picked.asset_name, X64);

    // Mirror case: only the foreign asset exists, so nothing may be selected.
    let only_arm = vec![release("v2.0.0", false, &["windows-arm64.zip"])];
    assert!(select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        "64.zip",
        X64,
        only_arm
    )
    .is_none());
}

#[test]
fn an_arm_pattern_on_an_x64_installation_selects_nothing() {
    // The operator configured the ARM pattern on an x64 host: the pattern may
    // narrow the search but may not widen it beyond the running architecture.
    let both = vec![release(
        "v2.0.0",
        false,
        &["windows-x64.zip", "windows-arm64.zip"],
    )];
    assert!(select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        "arm64.zip",
        X64,
        both
    )
    .is_none());
}

#[test]
fn each_architecture_selects_its_own_asset_from_a_dual_release() {
    let releases = vec![release(
        "v2.0.0",
        false,
        &["windows-arm64.zip", "windows-x64.zip"],
    )];
    let x64 = select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        X64,
        X64,
        releases.clone(),
    )
    .unwrap();
    let arm = select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        ARM64,
        ARM64,
        releases,
    )
    .unwrap();
    assert_eq!(x64.asset_name, X64);
    assert_eq!(arm.asset_name, ARM64);
}

#[test]
fn a_pattern_that_is_not_a_suffix_yields_no_update() {
    let releases = vec![release(
        "v2.0.0",
        false,
        &["guidance-runtime-windows-x64.zip"],
    )];
    // A prefix no longer matches: "guidance-runtime" is not a suffix.
    assert!(select(
        "owner/repo",
        Channel::Stable,
        &v("1.0.0"),
        "guidance-runtime",
        X64,
        releases
    )
    .is_none());
}

#[test]
fn an_explicit_pattern_is_used_verbatim() {
    let cfg = UpdateTarget {
        asset_pattern: Some("windows-arm64.zip".into()),
        ..config(Channel::Stable)
    };
    assert_eq!(cfg.resolved_asset_pattern().unwrap(), "windows-arm64.zip");
}

#[test]
fn an_omitted_pattern_derives_from_the_running_architecture() {
    let cfg = UpdateTarget {
        asset_pattern: None,
        ..config(Channel::Stable)
    };
    // Pinned literally per target so the test cannot pass by mirroring the
    // implementation it is meant to verify.
    let expected = if cfg!(target_arch = "x86_64") {
        X64
    } else if cfg!(target_arch = "aarch64") {
        ARM64
    } else {
        panic!("test host architecture is not part of the mapping")
    };
    assert_eq!(cfg.resolved_asset_pattern().unwrap(), expected);
}

#[test]
fn a_blank_pattern_is_rejected() {
    for blank in ["", "   ", "\t"] {
        let cfg = UpdateTarget {
            asset_pattern: Some(blank.into()),
            ..config(Channel::Stable)
        };
        let err = cfg.resolved_asset_pattern().unwrap_err();
        assert!(err.to_string().contains("asset_pattern"), "{blank:?}");
    }
}

#[test]
fn a_pattern_without_the_zip_suffix_is_rejected() {
    let cfg = UpdateTarget {
        asset_pattern: Some("windows-x64".into()),
        ..config(Channel::Stable)
    };
    let err = cfg.resolved_asset_pattern().unwrap_err();
    assert!(err.to_string().contains(".zip"), "{err}");
}

#[test]
fn a_pattern_with_surrounding_whitespace_is_trimmed() {
    let cfg = UpdateTarget {
        asset_pattern: Some("  windows-x64.zip ".into()),
        ..config(Channel::Stable)
    };
    assert_eq!(cfg.resolved_asset_pattern().unwrap(), X64);
}

#[test]
fn changing_the_channel_preserves_an_explicit_asset_pattern() {
    // Saving and reloading must not drop an operator override.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("updates.toml");
    let cfg = crate::UpdateConfig::from_file(crate::ConfigFile {
        repository: Some("owner/repo".into()),
        asset_pattern: Some(ARM64.into()),
        ..Default::default()
    })
    .unwrap();
    cfg.save(&path).unwrap();

    let mut loaded = crate::UpdateConfig::load(&path).unwrap();
    crate::set_or_get_channel(&mut loaded, None, Some(Channel::Beta)).unwrap();
    loaded.save(&path).unwrap();

    let reloaded = crate::UpdateConfig::load(&path).unwrap();
    let target = reloaded.single_target().unwrap();
    assert_eq!(target.channel, Channel::Beta);
    assert_eq!(target.asset_pattern.as_deref(), Some(ARM64));
}

#[test]
fn channel_parsing_rejects_unknown_names() {
    assert_eq!("stable".parse::<Channel>().unwrap(), Channel::Stable);
    assert!("nightly".parse::<Channel>().is_err());
}

#[test]
fn up_to_date_message_names_the_channel() {
    assert_eq!(
        crate::UpdateOutcome::UpToDate(Channel::Beta).message(),
        "No update available on beta channel"
    );
    assert_eq!(
        crate::UpdateOutcome::UpToDate(Channel::Stable).message(),
        "No update available on stable channel"
    );
}

// --- check_all fan-out -----------------------------------------------------

use crate::{TargetState, Visibility};

fn target(name: &str, visibility: Visibility) -> UpdateTarget {
    UpdateTarget {
        name: name.into(),
        repository: "owner/repo".into(),
        visibility,
        credential_entry: if visibility == Visibility::Private {
            Some("guidance-update".into())
        } else {
            None
        },
        channel: Channel::Stable,
        install_dir: format!("/tmp/{name}").into(),
        service_name: None,
        asset_pattern: Some("windows-x64.zip".into()),
    }
}

fn cfg_of(targets: Vec<UpdateTarget>) -> crate::UpdateConfig {
    crate::UpdateConfig::from_config_targets(targets)
}

/// Names the target `guidance-runtime` so it reports the running version as its
/// installed version. A target under any other name is reported as
/// `not_installed` until the per-target marker lands (T028).
fn runtime_target(visibility: Visibility) -> UpdateTarget {
    runtime_target_named(crate::LEGACY_TARGET_NAME, visibility)
}

fn runtime_target_named(name: &str, visibility: Visibility) -> UpdateTarget {
    UpdateTarget {
        name: name.into(),
        repository: "owner/repo".into(),
        visibility,
        credential_entry: if visibility == Visibility::Private {
            Some("guidance-update".into())
        } else {
            None
        },
        channel: Channel::Stable,
        install_dir: format!("/tmp/{name}").into(),
        service_name: Some("GuidanceRuntime".into()),
        asset_pattern: Some("windows-x64.zip".into()),
    }
}

/// FR-008 / SC-002: with two targets configured, both are checked even when
/// the first one fails, and the run reports one result per target.
#[tokio::test]
async fn check_all_reports_one_result_per_target_and_isolates_failures() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let cfg = cfg_of(vec![
        runtime_target(Visibility::Public),
        target("second-target", Visibility::Public),
    ]);

    let store = FixedCredentialStore(None);
    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(run.targets.len(), 2, "n targets must yield n checks");
    assert_eq!(run.targets[0].target, crate::LEGACY_TARGET_NAME);
    assert_eq!(run.targets[1].target, "second-target");
    // A foreign target without a version marker is reported honestly rather
    // than borrowing the runtime's version (T028 supplies the real marker).
    assert_eq!(run.targets[1].state, TargetState::NotInstalled);
}

/// A target that is not this binary's own product reports `not_installed` and
/// never claims the running crate version as its installed version.
#[tokio::test]
async fn a_foreign_target_is_not_installed_and_carries_no_version() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let cfg = cfg_of(vec![target("guidance", Visibility::Public)]);

    let store = FixedCredentialStore(None);
    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    let check = &run.targets[0];
    assert_eq!(check.state, TargetState::NotInstalled);
    assert_eq!(check.installed_version, None);
    assert_eq!(check.offered_version, None);
    // Not installed is not a failure: nothing is wrong, there is just no local
    // installation to compare against.
    assert_eq!(run.exit_status(), 0);
}

/// A private target whose credential entry is absent reports the pinned phrase.
/// The store double returns no secret, so this exercises the real credential
/// path rather than an interim rule; the unconditional private branch it once
/// covered no longer exists.
#[tokio::test]
async fn a_private_target_reports_inaccessible_with_the_verbatim_phrase() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let cfg = cfg_of(vec![runtime_target(Visibility::Private)]);

    let store = FixedCredentialStore(None);
    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    let check = &run.targets[0];
    assert_eq!(check.state, TargetState::Inaccessible);
    assert!(check
        .detail
        .as_deref()
        .unwrap_or_default()
        .starts_with(crate::report::PHRASE));
    assert!(
        check.offered_version.is_none(),
        "an inaccessible target must not advertise an offered version"
    );
    assert_eq!(run.exit_status(), 2);
}

/// A public target with a newer permitted release reports the offered version,
/// proving the fan-out wires selection through to the aggregate.
#[tokio::test]
async fn a_public_target_with_a_newer_release_reports_update_available() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let cfg = cfg_of(vec![runtime_target(Visibility::Public)]);

    let store = FixedCredentialStore(None);
    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(run.targets[0].state, TargetState::UpdateAvailable);
    assert_eq!(run.targets[0].offered_version.as_deref(), Some("2.0.0"));
    assert_eq!(run.targets[0].installed_version.as_deref(), Some("1.0.0"));
    assert_eq!(run.exit_status(), 0);
}

/// No newer release means up to date, which is distinct from inaccessible.
#[tokio::test]
async fn a_public_target_with_no_newer_release_is_up_to_date() {
    let source = FixedReleases(vec![release("v1.0.0", false, &[X64])]);
    let cfg = cfg_of(vec![runtime_target(Visibility::Public)]);

    let store = FixedCredentialStore(None);
    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(run.targets[0].state, TargetState::UpToDate);
    assert_eq!(run.exit_status(), 0);
}

/// Exit status 2 for a partial failure (FR-026): one target reports an access
/// problem while another is perfectly fine.
#[tokio::test]
async fn a_run_with_a_mixed_outcome_exits_two() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let cfg = cfg_of(vec![
        runtime_target(Visibility::Public),
        runtime_target_named("private-runtime", Visibility::Private),
    ]);

    let store = FixedCredentialStore(None);
    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(run.targets[0].state, TargetState::UpdateAvailable);
    assert_eq!(run.targets[1].state, TargetState::Inaccessible);
    assert_eq!(run.exit_status(), 2);
}

/// SC-002 in the direction that matters: the FIRST source unreachable must not
/// stop the SECOND from being checked. This drives the source-error branch,
/// which FixedReleases cannot reach.
#[tokio::test]
async fn an_unreachable_first_source_does_not_stop_the_second() {
    let mut failing = runtime_target(Visibility::Public);
    failing.repository = "broken/repo".into();
    let mut healthy = runtime_target_named("second-runtime", Visibility::Public);
    healthy.repository = "healthy/repo".into();

    let source = FailingFor("broken/repo", vec![release("v2.0.0", false, &[X64])]);
    let cfg = cfg_of(vec![failing, healthy]);

    let store = FixedCredentialStore(None);
    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(run.targets.len(), 2);
    assert_eq!(run.targets[0].state, TargetState::NetworkError);
    assert_eq!(run.targets[1].target, "second-runtime");
    assert_eq!(
        run.targets[1].state,
        TargetState::NotInstalled,
        "the second target must still be reached after the first fails"
    );
}

/// The routing predicate must also say no to an explicit [[targets]] file with
/// a single entry: such a file asks for the per-target shape even though it
/// holds one target.
#[tokio::test]
async fn an_explicit_targets_file_uses_the_per_target_shape() {
    let cfg = crate::UpdateConfig::parse(
        "[[targets]]\nname = \"solo\"\nrepository = \"A/A\"\ninstall_dir = \"/a\"\n",
    )
    .unwrap();

    assert!(
        !crate::uses_legacy_output(&cfg, None),
        "an explicit [[targets]] file must not fall back to the legacy wording"
    );
}

// --- legacy equivalence (SC-001) -------------------------------------------

/// SC-001: a configuration containing only the legacy flat keys must produce
/// exactly the outcome the pre-feature code produced, and the CLI must route
/// such a file through that path rather than through per-target rendering.
///
/// The message strings are pinned literally against the pre-change binary's
/// output, not derived from the implementation, so a wording drift fails the
/// test instead of following it.
#[tokio::test]
async fn a_legacy_flat_configuration_produces_the_historical_outcome() {
    let cfg =
        crate::UpdateConfig::parse("repository = \"owner/repo\"\nchannel = \"stable\"\n").unwrap();

    // The single target is the historical one, under the historical name.
    let only = cfg.single_target().unwrap();
    assert_eq!(only.name, crate::LEGACY_TARGET_NAME);
    assert_eq!(only.channel, Channel::Stable);
    // The flag the CLI branches on to keep the historical output contract.
    // This is the routing decision itself, not a model-level detail: an
    // independent review showed that a mutation of the per-target renderer
    // passed every other test while the binary printed different text, because
    // nothing pinned the choice between the two output paths.
    assert!(
        crate::uses_legacy_output(&cfg, None),
        "a flat file must be recognised as the legacy shape"
    );
    assert!(
        !crate::uses_legacy_output(&cfg, Some(crate::LEGACY_TARGET_NAME)),
        "naming a target is an explicit request for the per-target shape"
    );

    // Up to date: the exact pre-change wording, including the channel name.
    let source = FixedReleases(vec![release("v1.0.0", false, &[X64])]);
    let outcome = crate::check_update_with(&source, only, None, &v("1.0.0"))
        .await
        .unwrap();
    assert_eq!(outcome.message(), "No update available on stable channel");

    // Update available: the exact pre-change wording for the other branch.
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let outcome = crate::check_update_with(&source, only, None, &v("1.0.0"))
        .await
        .unwrap();
    assert_eq!(
        outcome.message(),
        "Update available: 1.0.0 -> 2.0.0 (stable)"
    );
}

// --- credential port -------------------------------------------------------

/// A secret must not render its value: every error path formats its context,
/// so a derived Debug would eventually put a token into a log line.
#[test]
fn a_secret_never_renders_its_value() {
    let secret = Secret::new("ghp_sentinel_do_not_print");
    assert!(!format!("{secret:?}").contains("sentinel"), "{secret:?}");
    assert!(!format!("{secret}").contains("sentinel"), "{secret}");
    assert!(format!("{secret:?}").contains("redacted"));
    assert_eq!(secret.expose(), "ghp_sentinel_do_not_print");
}

/// An absent entry and an unusable store must be distinguishable: the remedy
/// differs between provisioning an entry and fixing the store.
#[tokio::test]
async fn an_absent_entry_and_an_unavailable_store_are_different_shapes() {
    let absent = FixedCredentialStore(None).get("guidance").await;
    assert!(matches!(absent, Ok(None)), "{absent:?}");

    let unavailable = UnavailableCredentialStore.get("guidance").await;
    assert!(
        matches!(unavailable, Err(CredentialStoreError::Unavailable(_))),
        "{unavailable:?}"
    );
}

#[tokio::test]
async fn a_present_entry_yields_the_secret() {
    let store = FixedCredentialStore(Some(Secret::new("token-value")));
    let got = store.get("guidance").await.unwrap();
    assert_eq!(got.unwrap().expose(), "token-value");
}

/// The NoCredentialStore double is the proof that a public path never reads a
/// credential: it panics if it is ever consulted. This test pins the contract
/// of the double itself, so a future refactor cannot quietly make it a no-op.
#[tokio::test]
#[should_panic(expected = "must not consult the credential store")]
async fn the_no_credential_store_panics_when_consulted() {
    let _ = NoCredentialStore.get("guidance").await;
}

// --- authenticated private access (US2) ------------------------------------

fn private_runtime_target() -> UpdateTarget {
    let mut t = runtime_target(Visibility::Private);
    t.credential_entry = Some("guidance-update".into());
    t
}

/// FR-010: a private target with a valid credential is read successfully, and
/// the credential actually reaches the source adapter.
#[tokio::test]
async fn a_private_target_with_a_credential_reports_its_real_state() {
    let (source, seen) = RecordingReleases::new(vec![release("v2.0.0", false, &[X64])]);
    let store = FixedCredentialStore(Some(Secret::new("ghp_valid")));
    let cfg = cfg_of(vec![private_runtime_target()]);

    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(run.targets[0].state, TargetState::UpdateAvailable);
    assert_eq!(run.targets[0].offered_version.as_deref(), Some("2.0.0"));
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [Some("ghp_valid".to_string())],
        "the credential must reach the adapter"
    );
}

/// FR-012: a public target never consults the credential store. The store
/// double panics if it is touched, so this fails loudly if that changes.
#[tokio::test]
async fn a_public_target_never_consults_the_credential_store() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let store = NoCredentialStore;
    let cfg = cfg_of(vec![runtime_target(Visibility::Public)]);

    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(run.targets[0].state, TargetState::UpdateAvailable);
}

/// A public target sends no token even when a store would supply one.
#[tokio::test]
async fn a_public_target_sends_no_token() {
    let (source, seen) = RecordingReleases::new(vec![release("v2.0.0", false, &[X64])]);
    let store = FixedCredentialStore(Some(Secret::new("ghp_must_not_be_used")));
    let cfg = cfg_of(vec![runtime_target(Visibility::Public)]);

    let _ = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [None],
        "a public target must send no Authorization header"
    );
}

/// FR-013: an entry that is named but not provisioned is an access problem
/// carrying the verbatim phrase.
#[tokio::test]
async fn an_absent_credential_reports_inaccessible_with_the_phrase() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let store = FixedCredentialStore(None);
    let cfg = cfg_of(vec![private_runtime_target()]);

    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    let check = &run.targets[0];
    assert_eq!(check.state, TargetState::Inaccessible);
    assert!(check
        .detail
        .as_deref()
        .unwrap_or_default()
        .starts_with(crate::report::PHRASE));
    assert_eq!(run.exit_status(), 2);
}

/// An unusable store is a different outcome from an absent entry, because the
/// remedy differs.
#[tokio::test]
async fn an_unusable_store_reports_credential_store_unavailable() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let store = UnavailableCredentialStore;
    let cfg = cfg_of(vec![private_runtime_target()]);

    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(
        run.targets[0].state,
        TargetState::CredentialStoreUnavailable
    );
    assert_ne!(run.targets[0].state, TargetState::Inaccessible);
}

/// Operator decision: access is resolved before the installed-version
/// question, so a private target with a valid credential and no local
/// installation still reports the access outcome rather than a bare "not
/// installed". The credential here is absent, which is the case a user without
/// provisioned access actually hits.
#[tokio::test]
async fn access_problems_win_over_not_installed() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let store = FixedCredentialStore(None);
    // A foreign name means no installed version is known for it.
    let mut target = private_runtime_target();
    target.name = "guidance".into();
    let cfg = cfg_of(vec![target]);

    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(
        run.targets[0].state,
        TargetState::Inaccessible,
        "an operator without a credential must hear about access, not about installation"
    );
}

/// Refusal and non-existence must be indistinguishable (FR-014, SC-005).
#[test]
fn refusal_and_non_existence_classify_identically() {
    use crate::{classify_status, SourceError};
    use reqwest::StatusCode;

    let unauthorized = classify_status(StatusCode::UNAUTHORIZED);
    let forbidden = classify_status(StatusCode::FORBIDDEN);
    let missing = classify_status(StatusCode::NOT_FOUND);

    assert_eq!(unauthorized, SourceError::Inaccessible);
    assert_eq!(unauthorized, forbidden);
    assert_eq!(unauthorized, missing);
    // And the rendered text names no repository.
    assert_eq!(unauthorized.to_string(), "no access to update source");
    assert!(!unauthorized.to_string().contains("github.com"));
}

/// A source that answers with a refusal, so the downcast in
/// `classify_source_error` is exercised through `check_all` rather than only
/// through `classify_status`. Replacing the Inaccessible arm with NetworkError
/// must fail this test.
struct RefusingReleases;

impl Releases for RefusingReleases {
    fn list<'a>(&'a self, _access: Access<'a>) -> crate::BoxFuture<'a, Result<Vec<Release>>> {
        Box::pin(async move { Err(crate::SourceError::Inaccessible.into()) })
    }
}

/// FR-013/FR-015: a refusal arriving through the port is classified as an
/// access problem, not as a connectivity problem.
#[tokio::test]
async fn a_refusal_reaching_check_all_is_inaccessible_not_a_network_error() {
    let store = FixedCredentialStore(Some(Secret::new("ghp_valid")));
    let cfg = cfg_of(vec![runtime_target(Visibility::Public)]);

    let run = crate::check_all(&RefusingReleases, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(
        run.targets[0].state,
        TargetState::Inaccessible,
        "a refusal must not be reported as a connectivity problem"
    );
}

/// FR-013: a named but unprovisioned entry must tell the operator what to do.
/// This is the difference between "you were denied" and "nothing was ever
/// provisioned", and conflating them is what the review flagged.
#[tokio::test]
async fn an_absent_credential_names_the_entry_and_the_step() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let store = FixedCredentialStore(None);
    let cfg = cfg_of(vec![private_runtime_target()]);

    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    let detail = run.targets[0].detail.as_deref().unwrap_or_default();
    assert!(detail.starts_with(crate::report::PHRASE), "{detail}");
    assert!(detail.contains("guidance-update"), "{detail}");
    assert!(detail.contains("provision"), "{detail}");

    // And the rendered line shows it rather than suppressing it.
    let line = crate::render_check(&run.targets[0]);
    assert!(line.contains(crate::report::PHRASE), "{line}");
    assert!(line.contains("provision"), "{line}");
}

/// `AuthenticatedAsset` holds a token, so its Debug must delegate to the
/// redacting impl rather than render the struct field by field.
#[test]
fn an_authenticated_asset_never_renders_its_token() {
    let asset = AuthenticatedAsset {
        url: "https://api.github.com/repos/a/b/releases/assets/1".into(),
        token: Some(Secret::new("ghp_sentinel_asset")),
    };
    let rendered = format!("{asset:?}");
    assert!(!rendered.contains("sentinel"), "{rendered}");
    assert!(rendered.contains("redacted"), "{rendered}");
}

/// A target with no installed version still has its source read, so an access
/// problem is not reported as a clean "not installed" run. This is the case
/// where FR-015's four outcomes would otherwise collapse.
#[tokio::test]
async fn an_unreachable_source_on_an_uninstalled_target_is_not_a_clean_run() {
    let (source, _) = RecordingReleases::new(Vec::new());
    let store = FixedCredentialStore(None);
    let mut target = private_runtime_target();
    target.name = "not-on-this-machine".into();
    let cfg = cfg_of(vec![target]);

    let run = crate::check_all(&source, &store, &cfg, &v("1.0.0")).await;

    // No credential, so access fails before the source is even asked.
    assert_eq!(run.targets[0].state, TargetState::Inaccessible);
    assert_eq!(run.exit_status(), 2);
}

/// With a credential present but the source refusing, a target that is not
/// installed reports the access problem rather than a clean run.
#[tokio::test]
async fn a_refusal_on_an_uninstalled_target_is_reported_as_access() {
    let store = FixedCredentialStore(Some(Secret::new("ghp_valid")));
    let mut target = runtime_target(Visibility::Public);
    target.name = "not-on-this-machine".into();
    let cfg = cfg_of(vec![target]);

    let run = crate::check_all(&RefusingReleases, &store, &cfg, &v("1.0.0")).await;

    assert_eq!(run.targets[0].state, TargetState::Inaccessible);
    assert_eq!(run.exit_status(), 2);
}
