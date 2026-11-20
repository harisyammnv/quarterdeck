//! Status records: `state/<task>.status`.
//!
//! Each line is one append-only event written by a worker, in the form
//! `<state> [at=<epoch>] [key=<slug>]: <text>`.
//! Firstmate documents a status line as a wake event, not as current truth, so the UI labels
//! anything taken from here as the "latest event".

use serde::Serialize;

/// The live state of a task, as a closed set the UI can style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    Working,
    NeedsDecision,
    Blocked,
    Paused,
    Done,
    Failed,
    Unknown,
}

impl TaskState {
    /// Maps a state word from a status line or the snapshot. Unrecognised words are `Unknown`.
    pub fn from_word(word: &str) -> Self {
        match word.trim().to_ascii_lowercase().as_str() {
            "working" => Self::Working,
            "needs-decision" => Self::NeedsDecision,
            "blocked" => Self::Blocked,
            "paused" => Self::Paused,
            "done" => Self::Done,
            "failed" => Self::Failed,
            _ => Self::Unknown,
        }
    }
}

/// One parsed status line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StatusEvent {
    /// The first word of the line exactly as written, such as `working` or `resolved`.
    pub word: String,
    /// Unix seconds from `[at=...]`, when present and numeric.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub at: Option<i64>,
    /// The decision key from `[key=...]`, when present.
    pub key: Option<String>,
    /// Everything after the first `:` that follows the tags.
    pub text: String,
}

impl StatusEvent {
    pub fn state(&self) -> TaskState {
        TaskState::from_word(&self.word)
    }

    fn is_resolution(&self) -> bool {
        self.word.eq_ignore_ascii_case("resolved")
    }
}

/// Parses one status line. Returns `None` for a blank line.
pub fn parse_line(line: &str) -> Option<StatusEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let word_end = line
        .find(|c: char| c.is_whitespace() || c == ':' || c == '[')
        .unwrap_or(line.len());
    let word = line[..word_end].to_string();
    let mut rest = line[word_end..].trim_start();
    let mut at = None;
    let mut key = None;
    while let Some(tag) = rest.strip_prefix('[') {
        let Some(close) = tag.find(']') else { break };
        let (name, value) = tag[..close].split_once('=').unwrap_or((&tag[..close], ""));
        match name.trim() {
            "at" => at = value.trim().parse::<i64>().ok(),
            "key" if !value.trim().is_empty() => key = Some(value.trim().to_string()),
            _ => {}
        }
        rest = tag[close + 1..].trim_start();
    }
    let text = rest.strip_prefix(':').unwrap_or(rest).trim().to_string();
    Some(StatusEvent {
        word,
        at,
        key,
        text,
    })
}

/// The latest meaningful event in a status record.
///
/// `resolved` lines close a decision or blocker; they are not a state of their own.
/// When the last state line is `needs-decision`, `blocked` or `paused` and a `resolved`
/// line follows it, the task has resumed, so the result is `working` carrying the
/// resolution's time and text.
pub fn latest_event(content: &str) -> Option<StatusEvent> {
    let mut latest_resolution: Option<StatusEvent> = None;
    for line in content.lines().rev() {
        let Some(event) = parse_line(line) else {
            continue;
        };
        if event.is_resolution() {
            if latest_resolution.is_none() {
                latest_resolution = Some(event);
            }
            continue;
        }
        return Some(match (event.state(), latest_resolution) {
            (TaskState::NeedsDecision | TaskState::Blocked | TaskState::Paused, Some(res)) => {
                StatusEvent {
                    word: "working".into(),
                    at: res.at.or(event.at),
                    key: res.key,
                    text: res.text,
                }
            }
            _ => event,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_line() {
        let e = parse_line("working [at=1795168723]: setup done on fm/x from dev").unwrap();
        assert_eq!(e.word, "working");
        assert_eq!(e.at, Some(1_795_168_723));
        assert_eq!(e.key, None);
        assert_eq!(e.text, "setup done on fm/x from dev");
        assert_eq!(e.state(), TaskState::Working);
    }

    #[test]
    fn parses_key_and_at_in_any_order() {
        let e = parse_line("needs-decision [key=pick-a] [at=12]: A or B?").unwrap();
        assert_eq!(e.state(), TaskState::NeedsDecision);
        assert_eq!(e.key.as_deref(), Some("pick-a"));
        assert_eq!(e.at, Some(12));
        assert_eq!(e.text, "A or B?");
        let e = parse_line("blocked [at=13] [key=k]: stuck").unwrap();
        assert_eq!((e.at, e.key.as_deref()), (Some(13), Some("k")));
    }

    #[test]
    fn text_keeps_urls_and_colons() {
        let e = parse_line("done [at=1]: PR https://example.invalid/o/r/pull/7").unwrap();
        assert_eq!(e.text, "PR https://example.invalid/o/r/pull/7");
    }

    #[test]
    fn tolerates_missing_tags_and_bad_stamp() {
        let e = parse_line("paused: waiting on CI").unwrap();
        assert_eq!(
            (e.word.as_str(), e.at, e.text.as_str()),
            ("paused", None, "waiting on CI")
        );
        let e = parse_line("working [at=<epoch>]: x").unwrap();
        assert_eq!(e.at, None);
        assert!(parse_line("   ").is_none());
    }

    #[test]
    fn unknown_word_is_unknown_state() {
        assert_eq!(
            parse_line("teleporting [at=1]: x").unwrap().state(),
            TaskState::Unknown
        );
    }

    #[test]
    fn latest_is_last_line_even_without_trailing_newline() {
        let c = "working [at=1]: a\ndone [at=2]: b";
        assert_eq!(latest_event(c).unwrap().word, "done");
    }

    #[test]
    fn resolution_resumes_a_decision() {
        let c = "working [at=1]: a\nneeds-decision [key=k] [at=2]: A or B\nresolved [key=k] [at=3]: chose A\n";
        let e = latest_event(c).unwrap();
        assert_eq!(e.state(), TaskState::Working);
        assert_eq!(e.at, Some(3));
        assert_eq!(e.text, "chose A");
    }

    #[test]
    fn resolution_does_not_override_a_later_state() {
        let c = "needs-decision [at=2]: q\nresolved [at=3]: ok\ndone [at=4]: shipped\n";
        assert_eq!(latest_event(c).unwrap().state(), TaskState::Done);
    }

    #[test]
    fn resolution_alone_is_not_a_state() {
        assert!(latest_event("resolved [at=3]: ok\n").is_none());
        assert!(latest_event("").is_none());
    }
}
