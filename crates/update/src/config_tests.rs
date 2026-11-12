//! Behavioural tests for the multi-target configuration model.
//!
//! Driven entirely through the public configuration API — no network, no
//! filesystem outside temp dirs. The rules asserted here are the ones the
//! specification pins by name (FR-004, FR-005, FR-011b) and the legacy
//! equivalence that SC-001 depends on.

use crate::{UpdateConfig, UpdateTarget, Visibility, LEGACY_TARGET_NAME};

fn parse(text: &str) -> anyhow::Result<UpdateConfig> {
    UpdateConfig::parse(text)
}

/// The legacy flat file must keep working and map onto one target named
/// `guidance-runtime`, which is what SC-001 depends on.
#[test]
fn legacy_flat_configuration_yields_one_runtime_target() {
    let cfg = parse(
        r#"
repository = "ORG/guidance-runtime"
channel = "beta"
install_dir = "C:/Program Files/Guidance Runtime"
service_name = "GuidanceRuntime"
"#,
    )
    .unwrap();

    let targets = cfg.targets();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].name, LEGACY_TARGET_NAME);
    assert_eq!(targets[0].repository, "ORG/guidance-runtime");
    assert_eq!(targets[0].channel, crate::Channel::Beta);
    assert_eq!(targets[0].visibility, Visibility::Public);
    assert_eq!(targets[0].service_name.as_deref(), Some("GuidanceRuntime"));
}

/// Loading must not rewrite the file; an operator's comments and key order
/// survive until they ask for a change.
#[test]
fn loading_does_not_mutate_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("updates.toml");
    let original = "# keep me\nrepository = \"ORG/rt\"\nchannel = \"stable\"\n";
    std::fs::write(&path, original).unwrap();

    let _ = UpdateConfig::load(&path).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn a_multi_target_configuration_is_accepted() {
    let cfg = parse(
        r#"
[[targets]]
name = "guidance-runtime"
repository = "ORG/guidance-runtime"
install_dir = "C:/Program Files/Guidance Runtime"
service_name = "GuidanceRuntime"

[[targets]]
name = "guidance"
repository = "ORG/guidance-dist"
visibility = "private"
credential_entry = "guidance-update"
install_dir = "C:/Program Files/Guidance"
"#,
    )
    .unwrap();

    assert_eq!(cfg.target_names(), vec!["guidance-runtime", "guidance"]);
    assert_eq!(cfg.targets()[1].visibility, Visibility::Private);
    assert_eq!(
        cfg.targets()[1].credential_entry.as_deref(),
        Some("guidance-update")
    );
}

/// FR-004: every rejection names the offending field.
///
/// The assertion checks the message phrase, not merely that the word appears:
/// an unrelated TOML parse error mentioning the key would otherwise satisfy a
/// weaker `contains("targets")` check and hide a missing rule.
#[test]
fn an_empty_target_list_is_rejected_naming_the_field() {
    let err = parse("targets = []\n").unwrap_err().to_string();
    assert!(err.contains("'targets' is present but empty"), "{err}");
}

#[test]
fn a_duplicate_target_name_is_rejected_naming_the_name() {
    let err = parse(
        r#"
[[targets]]
name = "same"
repository = "A/B"
install_dir = "/a"

[[targets]]
name = "same"
repository = "C/D"
install_dir = "/b"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("same"), "{err}");
    assert!(err.contains("duplicate"), "{err}");
}

#[test]
fn an_empty_target_name_is_rejected_naming_the_field() {
    let err = parse(
        r#"
[[targets]]
name = "  "
repository = "A/B"
install_dir = "/a"
"#,
    )
    .unwrap_err()
    .to_string();
    // Pins the phrase, not the bare word: "install_dir for target" and
    // "is private but names no" also contain "name".
    assert!(err.contains("empty 'name'"), "{err}");
}

/// FR-005 / FR-013: a private target must name its credential entry. This is a
/// configuration error, distinct from an entry that is named but not
/// provisioned (which is an access problem at check time).
#[test]
fn a_private_target_without_a_credential_entry_is_rejected() {
    let err = parse(
        r#"
[[targets]]
name = "guidance"
repository = "ORG/guidance-dist"
visibility = "private"
install_dir = "C:/Program Files/Guidance"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("credential_entry"), "{err}");
}

#[test]
fn a_blank_credential_entry_is_rejected() {
    let err = parse(
        r#"
[[targets]]
name = "guidance"
repository = "ORG/guidance-dist"
visibility = "private"
credential_entry = "   "
install_dir = "C:/Program Files/Guidance"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("credential_entry"), "{err}");
}

/// Two targets sharing an install directory would let one overwrite the other,
/// which FR-020 forbids.
#[test]
fn a_duplicate_install_dir_is_rejected() {
    let err = parse(
        r#"
[[targets]]
name = "a"
repository = "A/A"
install_dir = "/same"

[[targets]]
name = "b"
repository = "B/B"
install_dir = "/same"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("install_dir"), "{err}");
}

#[test]
fn mixing_legacy_keys_and_targets_is_rejected() {
    let err = parse(
        r#"
repository = "ORG/rt"

[[targets]]
name = "guidance"
repository = "ORG/guidance-dist"
install_dir = "/a"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("The ambiguous field is 'targets'"), "{err}");
}

#[test]
fn a_blank_asset_pattern_is_rejected_naming_the_field() {
    let err = parse(
        r#"
repository = "ORG/rt"
asset_pattern = "  "
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("asset_pattern"), "{err}");
}

#[test]
fn an_asset_pattern_without_zip_is_rejected() {
    let err = parse(
        r#"
repository = "ORG/rt"
asset_pattern = "windows-x64"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains(".zip"), "{err}");
}

#[test]
fn an_empty_repository_is_rejected() {
    let err = parse(
        r#"
[[targets]]
name = "a"
repository = ""
install_dir = "/a"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("repository"), "{err}");
}

/// FR-011b: the credential entry is a plain name. The parsed schema must be
/// identical on every platform, so the platform-specific store can never leak
/// into the configuration an operator writes.
///
/// The assertion pins the exact key set rather than a blacklist: a future
/// field named `store = "keyring"` would slip past a blacklist of the strings
/// we happen to know about today.
#[test]
fn the_parsed_schema_carries_no_platform_specific_store_key() {
    use std::collections::BTreeSet;

    let cfg = parse(
        r#"
[[targets]]
name = "guidance"
repository = "ORG/guidance-dist"
visibility = "private"
credential_entry = "guidance-update"
install_dir = "/a"
"#,
    )
    .unwrap();

    let document: toml::Value =
        toml::from_str(&toml::to_string_pretty(&cfg.targets()[0]).unwrap()).unwrap();
    let keys: BTreeSet<String> = document
        .as_table()
        .expect("a target serializes as a table")
        .keys()
        .cloned()
        .collect();
    let expected: BTreeSet<String> = [
        "name",
        "repository",
        "visibility",
        "credential_entry",
        "channel",
        "install_dir",
    ]
    .iter()
    .map(|k| (*k).to_string())
    .collect();
    // TOML omits a `None`, so `service_name` and `asset_pattern` are absent for
    // a target that does not set them. The assertion pins exactly what an
    // operator sees, which is the surface a platform-specific key would leak
    // into.
    assert_eq!(keys, expected, "target schema drifted: {keys:?}");

    // The entry is a bare name in the schema. Note this test pins the SCHEMA
    // shape only: no validation can prove an operator did not paste a secret
    // into `credential_entry`, so the guarantee here is that the field is
    // documented and typed as a name, not that a secret could never appear.
    let entry = cfg.targets()[0].credential_entry.as_deref().unwrap();
    assert_eq!(entry, "guidance-update");
}

/// A single-target configuration is addressable as one target; a multi-target
/// one refuses to be silently collapsed to the first.
#[test]
fn single_target_is_refused_for_a_multi_target_configuration() {
    let cfg = parse(
        r#"
[[targets]]
name = "a"
repository = "A/A"
install_dir = "/a"

[[targets]]
name = "b"
repository = "B/B"
install_dir = "/b"
"#,
    )
    .unwrap();

    let err = cfg.single_target().unwrap_err().to_string();
    assert!(err.contains('a') && err.contains('b'), "{err}");
}

#[test]
fn the_channel_is_set_per_target_and_other_targets_keep_theirs() {
    let mut cfg = parse(
        r#"
[[targets]]
name = "a"
repository = "A/A"
install_dir = "/a"

[[targets]]
name = "b"
repository = "B/B"
install_dir = "/b"
"#,
    )
    .unwrap();

    cfg.set_target_channel("b", crate::Channel::Beta).unwrap();
    assert_eq!(cfg.targets()[0].channel, crate::Channel::Stable);
    assert_eq!(cfg.targets()[1].channel, crate::Channel::Beta);

    let err = cfg
        .set_target_channel("missing", crate::Channel::Beta)
        .unwrap_err()
        .to_string();
    assert!(err.contains("missing"), "{err}");
}

/// The default visibility is public, so an existing file that never mentions
/// visibility keeps requiring no credential.
#[test]
fn visibility_defaults_to_public() {
    let target = UpdateTarget {
        name: "a".into(),
        repository: "A/A".into(),
        visibility: Visibility::default(),
        credential_entry: None,
        channel: crate::Channel::Stable,
        install_dir: "/a".into(),
        service_name: None,
        asset_pattern: None,
    };
    assert_eq!(target.visibility, Visibility::Public);
}

/// FR-003: saving a file that was loaded in the legacy flat shape must write
/// the flat shape back. Rewriting it into `[[targets]]` on an unrelated change
/// (for example setting the channel) would silently reshape the operator's
/// file, which is exactly what the legay-form guarantee forbids.
#[test]
fn saving_a_legacy_configuration_keeps_the_flat_shape() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("updates.toml");
    std::fs::write(
        &path,
        "repository = \"ORG/rt\"\nchannel = \"stable\"\ninstall_dir = \"/a\"\n",
    )
    .unwrap();

    let mut cfg = UpdateConfig::load(&path).unwrap();
    crate::set_or_get_channel(&mut cfg, None, Some(crate::Channel::Beta)).unwrap();
    cfg.save(&path).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        !text.contains("[[targets]]"),
        "a legacy file must stay flat: {text}"
    );
    assert!(text.contains("repository = \"ORG/rt\""), "{text}");
    assert!(text.contains("channel = \"beta\""), "{text}");
}

/// The mirror case: a file that used `[[targets]]` is written back as an array,
/// so the two shapes never converge accidentally.
#[test]
fn saving_a_targets_configuration_keeps_the_array_shape() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("updates.toml");
    std::fs::write(
        &path,
        "[[targets]]\nname = \"a\"\nrepository = \"A/A\"\ninstall_dir = \"/a\"\n",
    )
    .unwrap();

    let mut cfg = UpdateConfig::load(&path).unwrap();
    crate::set_or_get_channel(&mut cfg, Some("a"), Some(crate::Channel::Beta)).unwrap();
    cfg.save(&path).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("[[targets]]"), "{text}");
    assert!(text.contains("channel = \"beta\""), "{text}");
}

/// `--target` must narrow the run itself. Selecting before the fan-out is what
/// makes the JSON document carry exactly one element and stops the updater from
/// reading sources or credential stores the operator excluded.
#[test]
fn select_target_narrows_to_exactly_one_target() {
    let cfg = parse(
        r#"
[[targets]]
name = "a"
repository = "A/A"
install_dir = "/a"

[[targets]]
name = "b"
repository = "B/B"
install_dir = "/b"
"#,
    )
    .unwrap();

    let only = cfg.select_target("b").unwrap();
    assert_eq!(only.target_names(), vec!["b"]);

    let err = cfg.select_target("missing").unwrap_err().to_string();
    assert!(err.contains("missing"), "{err}");
    assert!(err.contains('a') && err.contains('b'), "{err}");
}

/// FR-020 hazards are reachable by a typo: an ignored `instal_dir` would make a
/// second target fall back to the runtime's directory, and an ignored
/// `service_nam` would make it control the runtime's service. Reject unknown
/// keys inside a target instead of silently defaulting them.
#[test]
fn a_misspelled_target_key_is_rejected() {
    let err = format!(
        "{:#}",
        parse(
            r#"
[[targets]]
name = "a"
repository = "A/A"
instal_dir = "/a"
"#,
        )
        .unwrap_err()
    );
    assert!(err.contains("instal_dir"), "{err}");
}

/// A rollback snapshot is removed before it is recreated, so it must never be
/// another target's install directory. The old `with_extension("backup")`
/// derivation made `App.backup` the snapshot of `App.Prod`; this pins the
/// validation that makes the collision impossible.
#[test]
fn a_rollback_directory_may_not_collide_with_an_install_dir() {
    let err = parse(
        r#"
[[targets]]
name = "product-a"
repository = "A/A"
install_dir = "/apps/App.Prod.backup"

[[targets]]
name = "product-b"
repository = "B/B"
install_dir = "/apps/App.Prod"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("rollback directory"), "{err}");
    assert!(
        err.contains("product-b") || err.contains("product-a"),
        "{err}"
    );
}

/// FR-020: the `[[targets]]` form must state where each product is installed.
/// Only the legacy flat form keeps its historical optional default.
#[test]
fn a_target_without_install_dir_is_rejected_naming_the_field() {
    let err = format!(
        "{:#}",
        parse("[[targets]]\nname = \"a\"\nrepository = \"A/A\"\n").unwrap_err()
    );
    assert!(err.contains("install_dir"), "{err}");
}

#[test]
fn a_legacy_flat_form_still_defaults_the_install_dir() {
    let cfg = parse("repository = \"A/A\"\n").unwrap();
    assert_eq!(
        cfg.targets()[0].install_dir,
        std::path::PathBuf::from(r"C:\Program Files\Guidance Runtime")
    );
}
