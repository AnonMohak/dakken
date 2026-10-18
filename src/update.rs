//! `dakken --check-update`: ask GitHub for the latest release. It runs only
//! when asked, and it uses `curl`, so dakken needs no HTTP library.

use std::process::Command;

const LATEST: &str = "https://api.github.com/repos/AnonMohak/dakken/releases/latest";

/// A version such as "v0.2.0" or "0.2.0" as numbers.
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let mut parts = s.trim().trim_start_matches('v').split('.');
    let mut next = || parts.next()?.parse().ok();
    Some((next()?, next()?, next()?))
}

/// The "tag_name" value of the GitHub release JSON.
fn tag_name(json: &str) -> Option<&str> {
    let rest = &json[json.find("\"tag_name\"")? + "\"tag_name\"".len()..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    rest.split('"').next()
}

/// The message for this version and the latest release tag.
fn message(current: &str, latest: &str) -> String {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) if l > c => format!(
            "A new version is available: {latest} (you have v{current}).\n\
             Get it from https://github.com/AnonMohak/dakken/releases/latest\n"
        ),
        (Some(_), Some(_)) => format!("dakken v{current} is the latest version.\n"),
        _ => format!("error: cannot read the version \"{latest}\"\n"),
    }
}

/// Ask GitHub, and give the text to print.
pub fn check() -> String {
    let current = env!("CARGO_PKG_VERSION");
    let output = Command::new("curl")
        .args([
            "-sSfL",
            "--max-time",
            "10",
            "-H",
            "User-Agent: dakken",
            LATEST,
        ])
        .output();
    let json = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            return format!("error: cannot reach GitHub: {}\n", err.trim());
        }
        Err(e) => return format!("error: cannot run curl: {e}\n"),
    };
    match tag_name(&json) {
        Some(tag) => message(current, tag),
        None => "error: the GitHub answer has no release tag\n".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(parse_version("v0.10.2"), Some((0, 10, 2)));
        assert!(parse_version("v0.10.0") > parse_version("0.9.9"));
        assert_eq!(parse_version("main"), None);
    }

    #[test]
    fn tag_name_is_read_from_the_json() {
        let json = r#"{"url": "x", "tag_name" : "v0.3.0", "name": "v0.3.0"}"#;
        assert_eq!(tag_name(json), Some("v0.3.0"));
        assert_eq!(tag_name("{}"), None);
    }

    #[test]
    fn message_says_if_there_is_a_newer_version() {
        assert!(message("0.2.0", "v0.3.0").starts_with("A new version is available: v0.3.0"));
        assert_eq!(
            message("0.2.0", "v0.2.0"),
            "dakken v0.2.0 is the latest version.\n"
        );
        assert!(message("0.3.0", "v0.2.0").contains("is the latest version"));
        assert!(message("0.2.0", "nightly").starts_with("error"));
    }
}
