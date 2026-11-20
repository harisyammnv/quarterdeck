//! The one write path: a note to Firstmate through `bin/fm-inbox.sh note`.
//!
//! A request id makes the submission idempotent: a retried click with the same id replays the
//! original note instead of creating a second one.

use serde::{Deserialize, Serialize};

use crate::gateway::{self, Entry, GatewayError};
use crate::home::FmHome;

/// Longest note body the app will submit, in characters.
pub const MAX_NOTE_CHARS: usize = 4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum NoteOutcome {
    /// A new note was saved.
    Created,
    /// The request id was already used; the original note is returned.
    Replay,
}

/// What Firstmate's intake reported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NoteReceipt {
    pub outcome: NoteOutcome,
    /// Firstmate's note id.
    pub id: String,
    pub request_id: String,
    /// Firstmate was woken for this note. `false` means saved but not announced yet.
    pub announced: bool,
    /// Firstmate has already acknowledged (handled) the note.
    pub acknowledged: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum NoteError {
    #[error("the note is empty")]
    Empty,
    #[error("the note is longer than {MAX_NOTE_CHARS} characters")]
    TooLong,
    #[error("invalid request id")]
    BadRequestId,
    #[error(transparent)]
    Gateway(#[from] GatewayError),
    #[error("fm-inbox.sh failed ({}): {stderr}", crate::gateway::exit_label(*.code))]
    Failed { code: Option<i32>, stderr: String },
    #[error("fm-inbox.sh answered in an unexpected format: {0}")]
    Format(String),
}

#[derive(Deserialize)]
struct NoteJson {
    schema: String,
    outcome: NoteOutcome,
    id: String,
    #[serde(default)]
    announced: Option<bool>,
    #[serde(default)]
    acknowledged: bool,
}

/// The same rule `fm-inbox.sh` applies: 1 to 128 of `A-Za-z0-9._:-`, not starting with a dot.
pub fn valid_request_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
}

fn validate(text: &str, request_id: &str) -> Result<String, NoteError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(NoteError::Empty);
    }
    if text.chars().count() > MAX_NOTE_CHARS {
        return Err(NoteError::TooLong);
    }
    if !valid_request_id(request_id) {
        return Err(NoteError::BadRequestId);
    }
    Ok(text.to_string())
}

/// Reads `note --json` output. Exit 0 is a saved and announced (or acknowledged) note;
/// exit 3 is saved but not announced, which is still a receipt, not a failure.
pub fn interpret(out: &gateway::Output, request_id: &str) -> Result<NoteReceipt, NoteError> {
    match out.code {
        Some(0 | 3) => {}
        code => {
            return Err(NoteError::Failed {
                code,
                stderr: out.stderr.trim().to_string(),
            })
        }
    }
    let line = out
        .stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| NoteError::Format("no JSON line".into()))?;
    let parsed: NoteJson =
        serde_json::from_str(line).map_err(|e| NoteError::Format(e.to_string()))?;
    if parsed.schema != "fm-inbox-note.v1" {
        return Err(NoteError::Format(format!("schema {}", parsed.schema)));
    }
    Ok(NoteReceipt {
        outcome: parsed.outcome,
        id: parsed.id,
        request_id: request_id.to_string(),
        announced: parsed.announced.unwrap_or(false) && out.code == Some(0),
        acknowledged: parsed.acknowledged,
    })
}

/// Submits a note through Firstmate's intake.
pub async fn submit(home: &FmHome, text: &str, request_id: &str) -> Result<NoteReceipt, NoteError> {
    let text = validate(text, request_id)?;
    let entry = Entry::InboxNote {
        request_id: request_id.to_string(),
        text,
    };
    let out = gateway::run(home, &entry, entry.default_timeout()).await?;
    interpret(&out, request_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(code: i32, stdout: &str) -> gateway::Output {
        gateway::Output {
            code: Some(code),
            stdout: stdout.into(),
            stderr: "boom".into(),
        }
    }

    const CREATED: &str = r#"{"schema":"fm-inbox-note.v1","outcome":"created","id":"n-1","request_id":"qd-a","saved":true,"announced":true,"acknowledged":false,"path":"/x"}"#;

    #[test]
    fn created_and_announced() {
        let r = interpret(&out(0, CREATED), "qd-a").unwrap();
        assert_eq!(r.outcome, NoteOutcome::Created);
        assert!(r.announced);
        assert_eq!(r.id, "n-1");
    }

    #[test]
    fn saved_but_unannounced_is_a_receipt() {
        let json = CREATED.replace(r#""announced":true"#, r#""announced":false"#);
        let r = interpret(&out(3, &json), "qd-a").unwrap();
        assert!(!r.announced);
    }

    #[test]
    fn replay_is_recognised() {
        let json = CREATED.replace("created", "replay");
        assert_eq!(
            interpret(&out(0, &json), "qd-a").unwrap().outcome,
            NoteOutcome::Replay
        );
    }

    #[test]
    fn failure_carries_stderr() {
        let err = interpret(&out(1, ""), "qd-a").unwrap_err();
        assert!(matches!(err, NoteError::Failed { code: Some(1), ref stderr } if stderr == "boom"));
    }

    #[test]
    fn validation() {
        assert!(matches!(validate("  ", "qd-a"), Err(NoteError::Empty)));
        assert!(matches!(
            validate("hi", "bad id"),
            Err(NoteError::BadRequestId)
        ));
        assert!(matches!(validate("hi", ".x"), Err(NoteError::BadRequestId)));
        assert!(matches!(
            validate(&"x".repeat(MAX_NOTE_CHARS + 1), "a"),
            Err(NoteError::TooLong)
        ));
        assert_eq!(validate("  hi \n", "qd:1.a_b-c").unwrap(), "hi");
    }
}
