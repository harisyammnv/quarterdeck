//! Quarterdeck core: a read-only view of one Firstmate home.
//!
//! This crate has no window code. It knows how to:
//! - find and validate a Firstmate home ([`home`]),
//! - run the few allow-listed Firstmate entry points ([`gateway`]),
//! - parse the bearings snapshot ([`snapshot`]) and the per-task status and meta records
//!   ([`status`], [`meta`]),
//! - merge both into one fleet list ([`fleet`]),
//! - keep that list live with file events ([`engine`]),
//! - submit one inbox note through `bin/fm-inbox.sh` ([`inbox`]).
//!
//! It never writes into the Firstmate home itself. The only write path is the inbox note,
//! and that goes through Firstmate's own script.

pub mod engine;
pub mod fleet;
pub mod gateway;
pub mod home;
pub mod inbox;
pub mod meta;
pub mod snapshot;
pub mod status;
mod timefmt;

pub use engine::{Engine, EngineHandle};
pub use fleet::{FleetRow, FleetView, StateSource, UpdateCause};
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
