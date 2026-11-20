//! A task's status history: every line of `state/<task>.status`, newest first.
//!
//! The record is append-only and can grow without bound, so only its last
//! [`MAX_BYTES`] are read and at most [`MAX_EVENTS`] events are returned.

use serde::Serialize;

use super::{read_tail, TaskId};
use crate::home::FmHome;
use crate::status::{parse_line, StatusEvent};

/// The most bytes read from the end of a status record.
pub const MAX_BYTES: u64 = 256 * 1024;
/// The most events returned.
pub const MAX_EVENTS: usize = 500;

/// A task's recent status events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StatusHistory {
    pub task_id: String,
    /// Newest first.
    pub events: Vec<StatusEvent>,
    /// True when older events exist that are not included.
    pub truncated: bool,
}

/// Reads the status history of `id`. A task with no status record has an empty history.
pub fn read(home: &FmHome, id: &TaskId) -> std::io::Result<StatusHistory> {
    let path = home.state_dir().join(format!("{id}.status"));
    let (text, cut) = match std::fs::symlink_metadata(&path) {
        Ok(m) if m.is_file() => read_tail(&path, MAX_BYTES)?,
        Ok(_) => {
            return Err(std::io::Error::other(
                "the status record is not a regular file",
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (String::new(), false),
        Err(e) => return Err(e),
    };
    let mut events: Vec<StatusEvent> = text.lines().rev().filter_map(parse_line).collect();
    let truncated = cut || events.len() > MAX_EVENTS;
    events.truncate(MAX_EVENTS);
    Ok(StatusHistory {
        task_id: id.to_string(),
        events,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::super::testutil::fake_home;
    use super::*;

    fn id() -> TaskId {
        TaskId::parse("app-login-fix").unwrap()
    }

    #[test]
    fn empty_without_a_record() {
        let (_dir, home) = fake_home();
        let h = read(&home, &id()).unwrap();
        assert!(h.events.is_empty());
        assert!(!h.truncated);
    }

    #[test]
    fn newest_first_with_resolutions_kept() {
        let (dir, home) = fake_home();
        std::fs::write(
            dir.path().join("state/app-login-fix.status"),
            "working [at=10]: setup done\n\nneeds-decision [at=20] [key=k]: A or B?\nresolved [at=30] [key=k]: A\n",
        )
        .unwrap();
        let h = read(&home, &id()).unwrap();
        let words: Vec<_> = h.events.iter().map(|e| e.word.as_str()).collect();
        assert_eq!(words, ["resolved", "needs-decision", "working"]);
        assert_eq!(h.events[1].key.as_deref(), Some("k"));
    }

    #[test]
    fn bounds_events() {
        let (dir, home) = fake_home();
        let body: String = (0..MAX_EVENTS + 10)
            .map(|i| format!("working [at={i}]: step {i}\n"))
            .collect();
        std::fs::write(dir.path().join("state/app-login-fix.status"), body).unwrap();
        let h = read(&home, &id()).unwrap();
        assert_eq!(h.events.len(), MAX_EVENTS);
        assert!(h.truncated);
        assert_eq!(h.events[0].text, format!("step {}", MAX_EVENTS + 9));
    }

    #[test]
    fn refuses_a_linked_record() {
        let (dir, home) = fake_home();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("x"), "working: x\n").unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("x"),
            dir.path().join("state/app-login-fix.status"),
        )
        .unwrap();
        assert!(read(&home, &id()).is_err());
    }
}
