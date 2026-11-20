//! Launches of programs that are not Firstmate scripts, on their own allow-list.
//!
//! The Firstmate gateway ([`crate::gateway`]) stays pure: only Firstmate's scripts. Anything
//! else the app may start (the system browser, Orca's terminal switch, Kiri's UI) is one
//! variant here, with a fixed absolute binary, an argument vector, no shell, a cleared
//! environment, stdin closed and a timeout. Each variant validates its own input, and every
//! launch is started by a person pressing a button.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

/// Longest URL the app will hand to the browser.
pub const MAX_URL_LEN: usize = 2048;

/// The allow-listed launches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum External {
    /// `/usr/bin/open <url>`: opens an `https` link in the default browser.
    OpenUrl(String),
    /// `/usr/bin/open -b dev.kiri.app`: opens Kiri's own window. Takes no input at all.
    KiriUi,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExternalError {
    #[error("only https links can be opened")]
    NotHttps,
    #[error("the link is not a valid address")]
    BadUrl,
    #[error("links with a user name or password are not opened")]
    Credentials,
    #[error("the link is longer than {MAX_URL_LEN} characters")]
    TooLong,
    #[error("could not start {0}: {1}")]
    Spawn(&'static str, String),
    #[error("{0} did not finish in time")]
    Timeout(&'static str),
    #[error("{0} failed ({1})")]
    Failed(&'static str, String),
}

/// Checks a URL before it reaches the browser: `https` only, a host, no credentials, no
/// whitespace or control characters, and a length cap.
pub fn validate_url(url: &str) -> Result<(), ExternalError> {
    if url.len() > MAX_URL_LEN {
        return Err(ExternalError::TooLong);
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(ExternalError::BadUrl);
    }
    let Some(rest) = url
        .get(..8)
        .filter(|p| p.eq_ignore_ascii_case("https://"))
        .map(|_| &url[8..])
    else {
        return Err(ExternalError::NotHttps);
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return Err(ExternalError::Credentials);
    }
    let host = authority.rsplit_once(':').map_or(authority, |(h, port)| {
        if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            h
        } else {
            authority
        }
    });
    let host_ok = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '[' | ']' | ':'));
    if !host_ok {
        return Err(ExternalError::BadUrl);
    }
    Ok(())
}

impl External {
    /// The fixed binary.
    fn program(&self) -> &'static str {
        match self {
            Self::OpenUrl(_) | Self::KiriUi => "/usr/bin/open",
        }
    }

    /// The argument vector, after validation.
    pub fn args(&self) -> Result<Vec<String>, ExternalError> {
        match self {
            Self::OpenUrl(url) => {
                validate_url(url)?;
                Ok(vec![url.clone()])
            }
            Self::KiriUi => Ok(vec!["-b".into(), "dev.kiri.app".into()]),
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(10)
    }
}

/// Validates and runs one allow-listed launch, waiting for it to finish.
pub async fn run(launch: &External) -> Result<(), ExternalError> {
    let program = launch.program();
    let args = launch.args()?;
    let mut cmd = Command::new(program);
    cmd.env_clear();
    for key in ["HOME", "USER", "LOGNAME", "TMPDIR", "LANG"] {
        if let Some(v) = std::env::var_os(key) {
            cmd.env(key, v);
        }
    }
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let child = cmd
        .spawn()
        .map_err(|e| ExternalError::Spawn(program, e.to_string()))?;
    match tokio::time::timeout(launch.timeout(), child.wait_with_output()).await {
        Ok(Ok(out)) if out.status.success() => Ok(()),
        Ok(Ok(out)) => Err(ExternalError::Failed(
            program,
            // The program's own error text is not shown: only the exit status.
            out.status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string()),
        )),
        Ok(Err(e)) => Err(ExternalError::Spawn(program, e.to_string())),
        Err(_) => Err(ExternalError::Timeout(program)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_url_argv_is_the_url_alone() {
        let e = External::OpenUrl("https://forge.example.invalid/acme/app/pull/7".into());
        assert_eq!(e.program(), "/usr/bin/open");
        assert_eq!(
            e.args().unwrap(),
            vec!["https://forge.example.invalid/acme/app/pull/7".to_string()]
        );
    }

    #[test]
    fn kiri_ui_argv_is_fixed() {
        let e = External::KiriUi;
        assert_eq!(e.program(), "/usr/bin/open");
        assert_eq!(
            e.args().unwrap(),
            vec!["-b".to_string(), "dev.kiri.app".to_string()]
        );
    }

    #[test]
    fn accepts_plain_https_links() {
        for ok in [
            "https://example.invalid",
            "HTTPS://example.invalid/a?b=c#d",
            "https://example.invalid:8443/x",
            "https://[::1]/x",
        ] {
            assert_eq!(validate_url(ok), Ok(()), "{ok}");
        }
    }

    #[test]
    fn refuses_everything_else() {
        assert_eq!(
            validate_url("http://example.invalid"),
            Err(ExternalError::NotHttps)
        );
        assert_eq!(
            validate_url("file:///etc/passwd"),
            Err(ExternalError::NotHttps)
        );
        assert_eq!(
            validate_url("javascript:alert(1)"),
            Err(ExternalError::NotHttps)
        );
        assert_eq!(
            validate_url("-a https://x.invalid"),
            Err(ExternalError::BadUrl)
        );
        assert_eq!(
            validate_url("https://user:pw@example.invalid/"),
            Err(ExternalError::Credentials)
        );
        assert_eq!(validate_url("https://"), Err(ExternalError::BadUrl));
        assert_eq!(
            validate_url("https://exa mple.invalid"),
            Err(ExternalError::BadUrl)
        );
        assert_eq!(
            validate_url("https://example.invalid/\nx"),
            Err(ExternalError::BadUrl)
        );
        assert_eq!(
            validate_url("https://ex%41mple.invalid/"),
            Err(ExternalError::BadUrl)
        );
        let long = format!("https://example.invalid/{}", "a".repeat(MAX_URL_LEN));
        assert_eq!(validate_url(&long), Err(ExternalError::TooLong));
    }
}
