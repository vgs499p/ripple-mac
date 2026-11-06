// SPDX-License-Identifier: GPL-2.0-or-later
use std::sync::OnceLock;

/// Native client identity with conventional OS tokens for server session parsers.
pub fn user_agent() -> &'static str {
    static VALUE: OnceLock<String> = OnceLock::new();
    VALUE.get_or_init(|| {
        #[cfg(target_os = "macos")]
        let platform = {
            let version = std::process::Command::new("/usr/bin/sw_vers")
                .arg("-productVersion")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok());
            mac_platform(version.as_deref(), std::env::consts::ARCH)
        };
        #[cfg(not(target_os = "macos"))]
        let platform = format!("{}; {}", std::env::consts::OS, std::env::consts::ARCH);
        format!("RippleNative/{} ({platform})", env!("CARGO_PKG_VERSION"))
    })
}

#[cfg(any(target_os = "macos", test))]
fn mac_platform(version: Option<&str>, arch: &str) -> String {
    let cpu = if arch == "aarch64" { "ARM" } else { "Intel" };
    let version = version.map(str::trim).filter(|value| {
        !value.is_empty()
            && value.len() <= 20
            && value
                .split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|c| c.is_ascii_digit()))
    });
    match version {
        Some(version) => format!(
            "Macintosh; {cpu} Mac OS X {}; {arch}",
            version.replace('.', "_")
        ),
        None => format!("Macintosh; macOS; {arch}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_reports_client_and_parseable_os() {
        assert_eq!(
            mac_platform(Some("27.2\n"), "aarch64"),
            "Macintosh; ARM Mac OS X 27_2; aarch64"
        );
        assert_eq!(
            mac_platform(Some("13.6.1"), "x86_64"),
            "Macintosh; Intel Mac OS X 13_6_1; x86_64"
        );
        assert!(user_agent().starts_with(concat!("RippleNative/", env!("CARGO_PKG_VERSION"))));
        assert!(!user_agent().contains("Safari/"));
    }
    #[test]
    fn malformed_version_does_not_create_an_invalid_header() {
        for version in [None, Some(""), Some("27..2"), Some("27.2\r\nInjected: yes")] {
            assert_eq!(
                mac_platform(version, "aarch64"),
                "Macintosh; macOS; aarch64"
            );
        }
    }
}
