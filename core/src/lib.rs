//! Quarterdeck core: a read-only view of one Firstmate home.
//!
//! This crate has no window code. It knows how to:
//! - find and validate a Firstmate home ([`home`]) and remember the one chosen in the
//!   window ([`config`]),
//! - run the few allow-listed Firstmate entry points ([`gateway`]),
//! - parse the fleet snapshot ([`snapshot`]) and the per-task status and meta records
//!   ([`status`], [`meta`]),
//! - merge both into one fleet list ([`fleet`]),
//! - keep that list live with file events ([`engine`]),
//! - submit one inbox note through `bin/fm-inbox.sh` ([`inbox`]).
//!
//! It never writes into the Firstmate home itself. The only write path is the inbox note,
//! and that goes through Firstmate's own script.
//! - read files under a fixed root, bounded ([`safe_read`]), and launch the few allow-listed
//!   non-Firstmate programs ([`external`]).
//!
//! Each app area adds one directory module here (`core/src/<area>/`) with a pure
//! `view(&HomeState)` projection or on-demand `list` and `read` readers.

pub mod config;
pub mod engine;
pub mod external;
pub mod fleet;
pub mod gateway;
pub mod home;
pub mod inbox;
pub mod memory;
pub mod meta;
pub mod reports;
pub mod safe_read;
pub mod skills;
pub mod snapshot;
pub mod status;
pub mod terminal;
mod timefmt;

pub use config::{AppConfig, HomeSource};
pub use engine::{Engine, EngineHandle, HomeState};
pub use fleet::{
    merged_state, FleetRow, FleetView, MergedState, SnapshotInfo, StateSource, UpdateCause,
};
pub use home::{FmHome, HomeError};
pub use inbox::{NoteError, NoteOutcome, NoteReceipt};
pub use status::{StatusEvent, TaskState};

/// Milliseconds since the Unix epoch, for latency probes.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
