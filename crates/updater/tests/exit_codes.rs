//! Exit-code contract for `guidance-updater` (update-cli.md §2):
//! `0` clean, `1` usage/configuration error, `2` at least one target failed.

use std::process::Command;

fn updater() -> Command {
    Command::new(env!("CARGO_BIN_EXE_guidance-updater"))
}

#[test]
fn help_exits_zero_on_stdout() {
    let out = updater().arg("--help").output().expect("spawn updater");
    assert!(out.status.success());
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage"), "help text expected on stdout");
}

#[test]
fn version_exits_zero() {
    let out = updater().arg("--version").output().expect("spawn updater");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.is_empty());
}

#[test]
fn unknown_flag_exits_one() {
    let out = updater()
        .arg("--definitely-not-a-flag")
        .output()
        .expect("spawn updater");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn invalid_format_value_exits_one() {
    let out = updater()
        .args(["check", "--format", "bogus"])
        .output()
        .expect("spawn updater");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn missing_config_file_exits_one() {
    let out = updater()
        .args(["--config", "Z:/definitely/missing/config.toml", "check"])
        .output()
        .expect("spawn updater");
    assert_eq!(out.status.code(), Some(1));
}
