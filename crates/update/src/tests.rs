//! Behavioural tests for the update pipeline, driven entirely through the
//! ports (no network, no filesystem outside temp dirs).

use crate::{
    check_with, download_and_verify_with, is_allowed, parse_checksum, select, Asset, Channel,
    Downloader, Release, Releases, UpdateTarget,
};
use anyhow::Result;
use semver::Version;

struct FixedReleases(Vec<Release>);

impl Releases for FixedReleases {
    fn list<'a>(&'a self, _repository: &'a str) -> crate::BoxFuture<'a, Result<Vec<Release>>> {
        Box::pin(async move { Ok(self.0.clone()) })
    }
}

/// Fails for one repository and succeeds for the other, so a test can drive
/// the source-error branch for exactly one target.
struct FailingFor(&'static str, Vec<Release>);

impl Releases for FailingFor {
    fn list<'a>(&'a self, repository: &'a str) -> crate::BoxFuture<'a, Result<Vec<Release>>> {
        let fails = repository == self.0;
        let releases = self.1.clone();
        Box::pin(async move {
            if fails {
                anyhow::bail!("simulated source failure for {repository}")
            }
            Ok(releases)
        })
    }
}

struct FixedDownloader(Vec<u8>, Vec<u8>);

impl Downloader for FixedDownloader {
    fn get<'a>(&'a self, url: &'a str) -> crate::BoxFuture<'a, Result<Vec<u8>>> {
        let bytes = if url.contains("sha256") {
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
            .map(|name| Asset {
                name: (*name).into(),
                browser_download_url: format!("https://example.test/{name}"),
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
    let picked = select(Channel::Stable, &v("1.0.0"), ASSET, ASSET, releases).unwrap();
    assert_eq!(picked.version, v("1.3.0"));
}

#[test]
fn selection_ignores_versions_not_above_current() {
    let releases = vec![release("v1.0.0", false, &[ASSET])];
    assert!(select(Channel::Stable, &v("1.0.0"), ASSET, ASSET, releases).is_none());
}

#[test]
fn selection_requires_a_matching_asset() {
    let releases = vec![release("v2.0.0", false, &["linux-x64.zip"])];
    assert!(select(Channel::Stable, &v("1.0.0"), ASSET, ASSET, releases).is_none());
}

#[test]
fn selection_attaches_the_checksum_url_when_present() {
    let releases = vec![release("v2.0.0", false, &[ASSET, "windows-x64.zip.sha256"])];
    let picked = select(Channel::Stable, &v("1.0.0"), ASSET, ASSET, releases).unwrap();
    assert_eq!(
        picked.checksum_url.as_deref(),
        Some("https://example.test/windows-x64.zip.sha256")
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
    let update = check_with(&source, &config(Channel::Stable), &v("1.0.0"))
        .await?
        .unwrap();
    let downloader = FixedDownloader(payload.to_vec(), checksum_text.as_bytes().to_vec());
    download_and_verify_with(&downloader, &update).await
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
    let update = check_with(&source, &config(Channel::Stable), &v("1.0.0"))
        .await
        .unwrap()
        .unwrap();
    let downloader = FixedDownloader(b"payload".to_vec(), Vec::new());
    let err = download_and_verify_with(&downloader, &update)
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
    let stable = check_with(&source, &config(Channel::Stable), &v("1.0.0"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stable.version, v("1.7.0"));

    let beta = check_with(&source, &config(Channel::Beta), &v("1.0.0"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(beta.version, v("1.9.0-beta.1"));
}

#[test]
fn x64_build_does_not_select_the_arm64_asset() {
    let releases = vec![release("v2.0.0", false, &["windows-arm64.zip"])];
    assert!(select(Channel::Stable, &v("1.0.0"), X64, X64, releases).is_none());
}

#[test]
fn arm64_build_does_not_select_the_x64_asset() {
    let releases = vec![release("v2.0.0", false, &["windows-x64.zip"])];
    assert!(select(Channel::Stable, &v("1.0.0"), ARM64, ARM64, releases).is_none());
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
    let picked = select(Channel::Stable, &v("1.0.0"), "64.zip", X64, arm_first).unwrap();
    assert_eq!(picked.asset_name, X64);

    // Mirror case: only the foreign asset exists, so nothing may be selected.
    let only_arm = vec![release("v2.0.0", false, &["windows-arm64.zip"])];
    assert!(select(Channel::Stable, &v("1.0.0"), "64.zip", X64, only_arm).is_none());
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
    assert!(select(Channel::Stable, &v("1.0.0"), "arm64.zip", X64, both).is_none());
}

#[test]
fn each_architecture_selects_its_own_asset_from_a_dual_release() {
    let releases = vec![release(
        "v2.0.0",
        false,
        &["windows-arm64.zip", "windows-x64.zip"],
    )];
    let x64 = select(Channel::Stable, &v("1.0.0"), X64, X64, releases.clone()).unwrap();
    let arm = select(Channel::Stable, &v("1.0.0"), ARM64, ARM64, releases).unwrap();
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

    let run = crate::check_all(&source, &cfg, &v("1.0.0")).await;

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

    let run = crate::check_all(&source, &cfg, &v("1.0.0")).await;

    let check = &run.targets[0];
    assert_eq!(check.state, TargetState::NotInstalled);
    assert_eq!(check.installed_version, None);
    assert_eq!(check.offered_version, None);
    // Not installed is not a failure: nothing is wrong, there is just no local
    // installation to compare against.
    assert_eq!(run.exit_status(), 0);
}

/// The private-target interim rule (superseded by T022). This test exists so a
/// refactor cannot silently turn a missing capability into "up to date", which
/// is exactly the misreport FR-013 forbids. When T022 lands, this test moves to
/// asserting the real credential path.
#[tokio::test]
async fn a_private_target_reports_inaccessible_with_the_verbatim_phrase() {
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let cfg = cfg_of(vec![runtime_target(Visibility::Private)]);

    let run = crate::check_all(&source, &cfg, &v("1.0.0")).await;

    let check = &run.targets[0];
    assert_eq!(check.state, TargetState::Inaccessible);
    assert_eq!(check.detail.as_deref(), Some("no access to update source"));
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

    let run = crate::check_all(&source, &cfg, &v("1.0.0")).await;

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

    let run = crate::check_all(&source, &cfg, &v("1.0.0")).await;

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

    let run = crate::check_all(&source, &cfg, &v("1.0.0")).await;

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

    let run = crate::check_all(&source, &cfg, &v("1.0.0")).await;

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
    let outcome = crate::check_update_with(&source, only, &v("1.0.0"))
        .await
        .unwrap();
    assert_eq!(outcome.message(), "No update available on stable channel");

    // Update available: the exact pre-change wording for the other branch.
    let source = FixedReleases(vec![release("v2.0.0", false, &[X64])]);
    let outcome = crate::check_update_with(&source, only, &v("1.0.0"))
        .await
        .unwrap();
    assert_eq!(
        outcome.message(),
        "Update available: 1.0.0 -> 2.0.0 (stable)"
    );
}
