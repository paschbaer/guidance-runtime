//! Architecture- and platform-aware asset naming.
//!
//! The release workflow publishes one archive per platform and architecture,
//! so an installation must ask for the archive that matches the platform and
//! architecture it is actually running on.
//!
//! Historically this table was Windows-only. It now covers every platform the
//! updater can run on, because on Linux or macOS the previous single match arm
//! failed before any network call was made — the update check could not even
//! start.

use anyhow::{bail, Result};

/// Every platform an archive is published for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

impl Platform {
    /// The platform the current process was built for.
    pub fn running() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Self::MacOs
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            // Linux and every other Unix-like target use the Linux asset,
            // which is the closest match the release workflow produces.
            Self::Linux
        }
    }

    /// The prefix used in archive names.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::MacOs => "macos",
            Self::Linux => "linux",
        }
    }
}

/// Every asset suffix the release workflow can produce.
///
/// This table is the single source for both the positive mapping and the
/// negative selection rule. It is maintained by hand and therefore covers
/// KNOWN platform/architecture pairs only — selection is defence in depth, not
/// a proof.
///
/// The Windows entries are byte-identical to the historical values; changing
/// them would make every installed Windows machine report "no matching
/// artifact" on its next update check.
pub const KNOWN_ARCH_SUFFIXES: [&str; 6] = [
    "windows-x64.zip",
    "windows-arm64.zip",
    "macos-x86_64.zip",
    "macos-aarch64.zip",
    "linux-x86_64.zip",
    "linux-aarch64.zip",
];

/// Maps a platform and a Rust target architecture name onto the release asset
/// suffix.
///
/// Both inputs are parameters rather than read from the environment so every
/// combination is testable on a single host.
pub fn asset_suffix_for(platform: Platform, arch: &str) -> Result<&'static str> {
    let suffix = match (platform, arch) {
        (Platform::Windows, "x86_64") => "windows-x64.zip",
        (Platform::Windows, "aarch64") => "windows-arm64.zip",
        (Platform::MacOs, "x86_64") => "macos-x86_64.zip",
        (Platform::MacOs, "aarch64") => "macos-aarch64.zip",
        (Platform::Linux, "x86_64") => "linux-x86_64.zip",
        (Platform::Linux, "aarch64") => "linux-aarch64.zip",
        (_, other) => bail!(
            "unsupported architecture '{other}' on {}: no release asset is published for it \
             (known architectures: x86_64, aarch64)",
            platform.as_str()
        ),
    };
    Ok(suffix)
}

/// Maps a Rust target architecture name onto the suffix for the running
/// platform.
///
/// Kept as the historical entry point so existing call sites keep working; it
/// resolves the platform itself. New code that needs a specific platform
/// should call [`asset_suffix_for`] instead.
pub fn asset_suffix_for_arch(arch: &str) -> Result<&'static str> {
    asset_suffix_for(Platform::running(), arch)
}

/// The architecture the current process was built for.
///
/// This is the only place that reads the environment for the architecture,
/// keeping [`asset_suffix_for`] pure.
pub fn running_arch() -> &'static str {
    std::env::consts::ARCH
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Windows mapping must not drift: installed machines compare the
    /// published asset name against these exact strings.
    #[test]
    fn windows_mapping_is_unchanged() {
        assert_eq!(
            asset_suffix_for(Platform::Windows, "x86_64").unwrap(),
            "windows-x64.zip"
        );
        assert_eq!(
            asset_suffix_for(Platform::Windows, "aarch64").unwrap(),
            "windows-arm64.zip"
        );
    }

    #[test]
    fn macos_and_linux_pairs_resolve() {
        assert_eq!(
            asset_suffix_for(Platform::MacOs, "x86_64").unwrap(),
            "macos-x86_64.zip"
        );
        assert_eq!(
            asset_suffix_for(Platform::MacOs, "aarch64").unwrap(),
            "macos-aarch64.zip"
        );
        assert_eq!(
            asset_suffix_for(Platform::Linux, "x86_64").unwrap(),
            "linux-x86_64.zip"
        );
        assert_eq!(
            asset_suffix_for(Platform::Linux, "aarch64").unwrap(),
            "linux-aarch64.zip"
        );
    }

    #[test]
    fn an_unknown_architecture_is_rejected_by_name() {
        let err = asset_suffix_for(Platform::Linux, "riscv64").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("riscv64"), "{msg}");
        assert!(msg.contains("unsupported"), "{msg}");
    }

    /// The error names the platform too, so an operator can tell a missing
    /// platform from a missing architecture.
    #[test]
    fn the_rejection_names_the_platform() {
        let err = asset_suffix_for(Platform::MacOs, "riscv64").unwrap_err();
        assert!(err.to_string().contains("macos"), "{err}");
    }

    #[test]
    fn the_mapping_is_case_exact() {
        assert!(asset_suffix_for(Platform::Windows, "X86_64").is_err());
        assert!(asset_suffix_for(Platform::Windows, "Aarch64").is_err());
    }

    #[test]
    fn every_known_architecture_maps_to_a_known_suffix() {
        for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
            for arch in ["x86_64", "aarch64"] {
                let suffix = asset_suffix_for(platform, arch).unwrap();
                assert!(
                    KNOWN_ARCH_SUFFIXES.contains(&suffix),
                    "{suffix} missing from KNOWN_ARCH_SUFFIXES"
                );
            }
        }
    }

    /// Guards against a host whose rustc arch name was never added to the
    /// mapping; the assertion is intentionally host-dependent. Before this
    /// change it failed on every non-Windows host.
    #[test]
    fn the_running_platform_and_architecture_map_to_a_known_suffix() {
        let suffix = asset_suffix_for_arch(running_arch()).unwrap();
        assert!(KNOWN_ARCH_SUFFIXES.contains(&suffix));
    }
}
