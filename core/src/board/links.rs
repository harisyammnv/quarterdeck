//! Link checks for anything the board passes to the UI to open.
//!
//! Recorded URLs and paths come from records workers write, so they are untrusted. A link
//! the UI may show is one the core's opener accepts; a report path is relative to the home,
//! stays inside `data/`, and has no `..`.

/// Longest report path kept, in bytes.
const MAX_PATH: usize = 512;

/// The URL when the core's opener would accept it (`external::validate_url`: https, a host,
/// no credentials, no whitespace or control characters, a length cap), else `None`. Using the
/// opener's own check means a link the board shows is a link that can be opened.
pub fn safe_web_url(raw: &str) -> Option<String> {
    let url = raw.trim();
    crate::external::validate_url(url)
        .ok()
        .map(|()| url.to_string())
}

/// The path when it is a relative path under `data/` with no parent steps, else `None`.
pub fn safe_report_path(raw: &str) -> Option<String> {
    let path = raw.trim();
    if path.is_empty()
        || path.len() > MAX_PATH
        || path.starts_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
    {
        return None;
    }
    let mut parts = path.split('/');
    if parts.next() != Some("data") {
        return None;
    }
    let mut depth = 0;
    for part in parts {
        if part.is_empty() || part == "." || part == ".." {
            return None;
        }
        depth += 1;
    }
    (depth > 0).then(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_https() {
        assert_eq!(
            safe_web_url(" https://forge.example.invalid/acme/app/pull/7 ").as_deref(),
            Some("https://forge.example.invalid/acme/app/pull/7")
        );
        assert!(safe_web_url("https://forge.example.invalid").is_some());
        assert!(safe_web_url("HTTPS://forge.example.invalid/x").is_some());
        assert!(safe_web_url("https://forge.example.invalid:8443/x").is_some());
    }

    #[test]
    fn refuses_other_schemes_and_tricks() {
        for bad in [
            "http://forge.example.invalid/x",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://",
            "https:///x",
            "https://user@evil.example.invalid/x",
            "https://forge.example.invalid/a b",
            "https://forge.example.invalid/\u{7}",
            "data:text/html,hi",
            "",
        ] {
            assert_eq!(safe_web_url(bad), None, "{bad}");
        }
        assert_eq!(
            safe_web_url(&format!("https://a.invalid/{}", "x".repeat(3000))),
            None
        );
    }

    #[test]
    fn report_paths_stay_in_data() {
        assert_eq!(
            safe_report_path("data/login-flake/report.md").as_deref(),
            Some("data/login-flake/report.md")
        );
        for bad in [
            "/data/x/report.md",
            "data/../config/secret",
            "data/./x",
            "data//x",
            "data/",
            "data",
            "state/x.status",
            "../data/x",
            "data\\x",
            "",
        ] {
            assert_eq!(safe_report_path(bad), None, "{bad}");
        }
    }
}
