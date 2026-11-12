//! Architecture-aware asset naming.
//!
//! The release workflow publishes one archive per Windows target
//! (`guidance-runtime-<version>-<arch>.zip`), so an installation must ask for
//! the archive that matches the architecture it is actually running on.

use anyhow::{bail, Result};

/// Every architecture suffix the release workflow currently produces.
///
/// This table is the single source for both the positive mapping and the
/// negative selection rule. It is maintained by hand and therefore covers
/// KNOWN architectures only — selection is defence in depth, not a proof.
pub const KNOWN_ARCH_SUFFIXES: [&str; 2] = ["windows-x64.zip", "windows-arm64.zip"];

/// Maps a Rust target architecture name onto the release asset suffix.
///
/// The architecture is a parameter rather than read from the environment so
/// the arm64 behaviour is testable on an x64 host.
pub fn asset_suffix_for_arch(arch: &str) -> Result<&'static str> {
    match arch {
        "x86_64" => Ok("windows-x64.zip"),
        "aarch64" => Ok("windows-arm64.zip"),
        other => bail!(
            "unsupported architecture '{other}': no release asset is published for it \
             (known: x86_64, aarch64)"
        ),
    }
}

/// The architecture the current process was built for.
///
/// This is the only place that reads the environment, keeping
/// [`asset_suffix_for_arch`] pure.
pub fn running_arch() -> &'static str {
    std::env::consts::ARCH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_64_maps_to_the_x64_asset() {
        assert_eq!(asset_suffix_for_arch("x86_64").unwrap(), "windows-x64.zip");
    }

    #[test]
    fn aarch64_maps_to_the_arm64_asset() {
        assert_eq!(
            asset_suffix_for_arch("aarch64").unwrap(),
            "windows-arm64.zip"
        );
    }

    #[test]
    fn an_unknown_architecture_is_rejected_by_name() {
        let err = asset_suffix_for_arch("riscv64").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("riscv64"), "{msg}");
        assert!(msg.contains("unsupported"), "{msg}");
    }

    #[test]
    fn the_mapping_is_case_exact() {
        assert!(asset_suffix_for_arch("X86_64").is_err());
        assert!(asset_suffix_for_arch("Aarch64").is_err());
    }

    #[test]
    fn every_known_architecture_maps_to_a_known_suffix() {
        for arch in ["x86_64", "aarch64"] {
            let suffix = asset_suffix_for_arch(arch).unwrap();
            assert!(
                KNOWN_ARCH_SUFFIXES.contains(&suffix),
                "{suffix} missing from KNOWN_ARCH_SUFFIXES"
            );
        }
    }

    #[test]
    fn the_running_architecture_maps_to_a_known_suffix() {
        // Guards against a host whose rustc arch name was never added to the
        // mapping; the assertion is intentionally host-dependent.
        let suffix = asset_suffix_for_arch(running_arch()).unwrap();
        assert!(KNOWN_ARCH_SUFFIXES.contains(&suffix));
    }
}
