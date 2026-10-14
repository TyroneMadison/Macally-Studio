//! Public version policy: "Major.Minor" with Minor 0-9 (1.0 … 1.9 → 2.0).
//! Tauri needs semver, so the manifests carry "X.Y.0" and the UI shows "X.Y".

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

/// Lenient parse for anything we may see: "1.0", "v1.0", "1.0.0", "1.2.0".
/// A patch component is accepted and ignored.
pub fn parse(s: &str) -> Option<Version> {
    let s = s.trim().strip_prefix('v').unwrap_or(s.trim());
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    if let Some(patch) = parts.next() {
        patch.parse::<u32>().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(Version { major, minor })
}

/// "1.0.0" -> "1.0". Unparseable input is returned unchanged.
pub fn display(semver: &str) -> String {
    match parse(semver) {
        Some(v) => format!("{}.{}", v.major, v.minor),
        None => semver.to_string(),
    }
}

/// Strict release-tag rule shared with scripts/release.mjs: "X.Y" with Y a single digit.
pub fn is_valid_release(s: &str) -> bool {
    let Some((major, minor)) = s.split_once('.') else {
        return false;
    };
    !major.is_empty()
        && major.bytes().all(|b| b.is_ascii_digit())
        && minor.len() == 1
        && minor.as_bytes()[0].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_forms() {
        let v = Version { major: 1, minor: 0 };
        assert_eq!(parse("1.0"), Some(v));
        assert_eq!(parse("v1.0"), Some(v));
        assert_eq!(parse("1.0.0"), Some(v));
        assert_eq!(parse("2.7.0"), Some(Version { major: 2, minor: 7 }));
        assert_eq!(parse("abc"), None);
        assert_eq!(parse("1"), None);
        assert_eq!(parse("1.0.0.0"), None);
    }

    #[test]
    fn displays_without_patch() {
        assert_eq!(display("1.0.0"), "1.0");
        assert_eq!(display("1.9.0"), "1.9");
        assert_eq!(display("garbage"), "garbage");
    }

    #[test]
    fn release_guard() {
        assert!(is_valid_release("1.0"));
        assert!(is_valid_release("1.9"));
        assert!(is_valid_release("10.3"));
        assert!(!is_valid_release("1.10"));
        assert!(!is_valid_release("1.0.1"));
        assert!(!is_valid_release("v1.0"));
        assert!(!is_valid_release("1."));
        assert!(!is_valid_release("1"));
    }

    #[test]
    fn orders_by_major_then_minor() {
        assert!(parse("1.9") < parse("2.0"));
        assert!(parse("1.1") > parse("1.0"));
    }
}
