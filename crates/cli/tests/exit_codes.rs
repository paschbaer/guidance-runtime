//! Exit-code contract for the `guidance` CLI (update-cli.md §2):
//! `0` clean, `1` usage/configuration error, `2` at least one target failed.

use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_guidance"))
}

#[test]
fn help_exits_zero_on_stdout() {
    let out = cli().arg("--help").output().expect("spawn guidance");
    assert!(out.status.success());
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage"), "help text expected on stdout");
}

#[test]
fn unknown_flag_exits_one() {
    let out = cli()
        .arg("--definitely-not-a-flag")
        .output()
        .expect("spawn guidance");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn invalid_format_value_exits_one() {
    let out = cli()
        .args(["update", "check", "--format", "bogus"])
        .output()
        .expect("spawn guidance");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn install_without_yes_exits_one() {
    let out = cli()
        .args(["update", "install"])
        .output()
        .expect("spawn guidance");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn missing_config_file_exits_one() {
    let out = cli()
        .args([
            "update",
            "check",
            "--config",
            "Z:/definitely/missing/config.toml",
        ])
        .output()
        .expect("spawn guidance");
    assert_eq!(out.status.code(), Some(1));
}

/// The global --json flag was removed (Option A): clap must reject it as an
/// unknown-flag usage error before any command runs.
#[test]
fn the_removed_global_json_flag_exits_one() {
    let out = cli()
        .args(["status", "--json"])
        .output()
        .expect("spawn guidance");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("unexpected argument"),
        "clap must name the removed flag as an unknown argument"
    );
}
