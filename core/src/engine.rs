//! The live engine: keeps a [`FleetView`] current from file events and snapshots.
//!
//! Read path, two speeds:
//! - fast: the home's `state/` directory is watched without recursion; a change to
//!   `<task>.status` or `<task>.meta` re-reads that one small record and publishes a new view
//!   at once, with no debounce, which is what keeps a status change under a second;
//! - true: `fm-bearings-snapshot.sh` runs once at start, then once after changes settle and at
//!   most once per `max_interval` while they continue. Only one run is ever in flight.
//!
//! Nothing here writes into the home.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use crate::fleet::{FleetModel, FleetView, UpdateCause};
use crate::gateway::{self, Entry};
use crate::home::FmHome;
use crate::inbox::{self, NoteError, NoteReceipt};
use crate::snapshot::Bearings;
use crate::{meta, now_ms, status};

/// Status records are read from the end, at most this many bytes.
const MAX_RECORD_BYTES: u64 = 64 * 1024;
/// How many recent update causes are kept for latency probes.
const PROBE_HISTORY: usize = 64;

#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Quiet time after the last change before a snapshot runs.
    pub settle: Duration,
    /// Longest wait for a snapshot while changes keep arriving.
    pub max_interval: Duration,
    /// Time budget for one snapshot run.
    pub snapshot_timeout: Duration,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            settle: Duration::from_millis(1500),
            max_interval: Duration::from_secs(10),
            snapshot_timeout: Entry::BearingsSnapshot.default_timeout(),
        }
    }
}

enum FsChange {
    Status(String, PathBuf),
    Meta(String, PathBuf),
    Backlog,
}

struct FsSignal {
    change: FsChange,
    detected_ms: i64,
}

enum Command {
    Refresh,
}

/// One end-to-end latency measurement for a status change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatencySample {
    pub task_id: String,
    pub seq: u32,
    pub file_mtime_ms: i64,
    pub detected_ms: i64,
    pub emitted_ms: i64,
    pub rendered_ms: i64,
}

impl LatencySample {
    /// From the status file being written to the row being painted.
    pub fn total_ms(&self) -> i64 {
        self.rendered_ms - self.file_mtime_ms
    }
}

/// A running engine. Dropping the handle stops the watcher and the loop.
pub struct EngineHandle {
    home: FmHome,
    views: watch::Receiver<FleetView>,
    commands: mpsc::UnboundedSender<Command>,
    probes: Arc<Mutex<Vec<(u32, UpdateCause)>>>,
    _watcher: notify::RecommendedWatcher,
}

impl EngineHandle {
    pub fn home(&self) -> &FmHome {
        &self.home
    }

    /// The latest view.
    pub fn current(&self) -> FleetView {
        self.views.borrow().clone()
    }

    /// A receiver that wakes on every published view.
    pub fn subscribe(&self) -> watch::Receiver<FleetView> {
        self.views.clone()
    }

    /// Asks for a snapshot run soon.
    pub fn refresh(&self) {
        let _ = self.commands.send(Command::Refresh);
    }

    /// Submits a note through Firstmate's intake.
    pub async fn submit_note(
        &self,
        text: &str,
        request_id: &str,
    ) -> Result<NoteReceipt, NoteError> {
        inbox::submit(&self.home, text, request_id).await
    }

    /// Completes a latency sample when the UI reports that it painted view `seq`.
    pub fn rendered(&self, seq: u32, rendered_ms: i64) -> Option<LatencySample> {
        let probes = self.probes.lock().ok()?;
        let (_, cause) = probes.iter().find(|(s, _)| *s == seq)?;
        Some(LatencySample {
            task_id: cause.task_id.clone(),
            seq,
            file_mtime_ms: cause.file_mtime_ms,
            detected_ms: cause.detected_ms,
            emitted_ms: cause.emitted_ms,
            rendered_ms,
        })
    }
}

/// Starts the engine.
pub struct Engine;

impl Engine {
    /// Starts watching `home` and publishing views. Must be called inside a Tokio runtime.
    pub fn start(home: FmHome, config: EngineConfig) -> notify::Result<EngineHandle> {
        let state_dir = home.state_dir();
        let backlog = home.backlog_file();
        let (fs_tx, fs_rx) = mpsc::unbounded_channel();
        let backlog_name = backlog.file_name().map(ToOwned::to_owned);
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let detected_ms = now_ms();
                let Ok(event) = res else { return };
                if matches!(event.kind, EventKind::Access(_)) {
                    return;
                }
                for path in event.paths {
                    if let Some(change) = classify(&path, backlog_name.as_deref()) {
                        let _ = fs_tx.send(FsSignal {
                            change,
                            detected_ms,
                        });
                    }
                }
            })?;
        watcher.watch(&state_dir, RecursiveMode::NonRecursive)?;
        if backlog.exists() {
            watcher.watch(&backlog, RecursiveMode::NonRecursive)?;
        }

        let mut model = FleetModel::new(home.label());
        let (view_tx, view_rx) = watch::channel(model.view(None));
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let probes = Arc::new(Mutex::new(Vec::new()));
        let task = Loop {
            home: home.clone(),
            config,
            model,
            views: view_tx,
            probes: probes.clone(),
        };
        tokio::spawn(task.run(fs_rx, cmd_rx));
        Ok(EngineHandle {
            home,
            views: view_rx,
            commands: cmd_tx,
            probes,
            _watcher: watcher,
        })
    }
}

/// Maps a changed path to what it means. Only direct children of `state/` count.
fn classify(path: &Path, backlog_name: Option<&std::ffi::OsStr>) -> Option<FsChange> {
    let parent = path.parent()?.file_name()?;
    let name = path.file_name()?;
    if parent == "data" && Some(name) == backlog_name {
        return Some(FsChange::Backlog);
    }
    if parent != "state" {
        return None;
    }
    let name = name.to_str()?;
    if name.starts_with('.') {
        return None;
    }
    if let Some(id) = name.strip_suffix(".status") {
        return Some(FsChange::Status(id.to_string(), path.to_path_buf()));
    }
    if let Some(id) = name.strip_suffix(".meta") {
        return Some(FsChange::Meta(id.to_string(), path.to_path_buf()));
    }
    None
}

/// Reads at most the last [`MAX_RECORD_BYTES`] of a record. `Ok(None)` when it is gone.
async fn read_tail(path: &Path) -> std::io::Result<Option<(String, i64)>> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let meta = file.metadata().await?;
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    let len = meta.len();
    let skip = len.saturating_sub(MAX_RECORD_BYTES);
    if skip > 0 {
        file.seek(std::io::SeekFrom::Start(skip)).await?;
    }
    let mut bytes = Vec::with_capacity(usize::try_from(len - skip).unwrap_or(0));
    file.read_to_end(&mut bytes).await?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if skip > 0 {
        // Drop the partial first line left by seeking into the middle of the file.
        text = text
            .split_once('\n')
            .map(|(_, rest)| rest.to_string())
            .unwrap_or_default();
    }
    Ok(Some((text, mtime_ms)))
}

struct Loop {
    home: FmHome,
    config: EngineConfig,
    model: FleetModel,
    views: watch::Sender<FleetView>,
    probes: Arc<Mutex<Vec<(u32, UpdateCause)>>>,
}

struct Schedule {
    settle_at: Option<Instant>,
    latest_at: Option<Instant>,
    running: bool,
    pending: bool,
}

impl Schedule {
    fn deadline(&self) -> Option<Instant> {
        match (self.settle_at, self.latest_at) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    fn changed(&mut self, now: Instant, config: &EngineConfig) {
        self.settle_at = Some(now + config.settle);
        self.latest_at.get_or_insert(now + config.max_interval);
    }
}

impl Loop {
    fn publish(&mut self, cause: Option<UpdateCause>) {
        let view = self.model.view(cause.clone());
        if let Some(cause) = cause {
            if let Ok(mut probes) = self.probes.lock() {
                probes.push((view.seq, cause));
                let excess = probes.len().saturating_sub(PROBE_HISTORY);
                probes.drain(..excess);
            }
        }
        self.views.send_replace(view);
    }

    async fn bootstrap(&mut self) {
        let Ok(mut dir) = tokio::fs::read_dir(self.home.state_dir()).await else {
            return;
        };
        while let Ok(Some(entry)) = dir.next_entry().await {
            match classify(&entry.path(), None) {
                Some(FsChange::Status(id, path)) => {
                    if let Ok(Some((text, _))) = read_tail(&path).await {
                        self.model.set_event(&id, status::latest_event(&text));
                    }
                }
                Some(FsChange::Meta(id, path)) => {
                    if let Ok(Some((text, _))) = read_tail(&path).await {
                        self.model.set_meta(&id, Some(meta::parse(&text)));
                    }
                }
                _ => {}
            }
        }
    }

    /// Applies one file change. Returns whether the fleet may have changed shape.
    async fn apply(&mut self, signal: FsSignal) -> bool {
        match signal.change {
            FsChange::Status(id, path) => {
                let (event, mtime_ms) = match read_tail(&path).await {
                    Ok(Some((text, mtime))) => (status::latest_event(&text), mtime),
                    Ok(None) => (None, signal.detected_ms),
                    Err(_) => return false,
                };
                if self.model.set_event(&id, event) {
                    let cause = UpdateCause {
                        task_id: id,
                        file_mtime_ms: mtime_ms,
                        detected_ms: signal.detected_ms,
                        emitted_ms: now_ms(),
                    };
                    self.publish(Some(cause));
                }
                true
            }
            FsChange::Meta(id, path) => {
                let meta = match read_tail(&path).await {
                    Ok(found) => found.map(|(text, _)| meta::parse(&text)),
                    Err(_) => return false,
                };
                if self.model.set_meta(&id, meta) {
                    self.publish(None);
                }
                true
            }
            FsChange::Backlog => true,
        }
    }

    async fn run(
        mut self,
        mut fs_rx: mpsc::UnboundedReceiver<FsSignal>,
        mut cmd_rx: mpsc::UnboundedReceiver<Command>,
    ) {
        self.bootstrap().await;
        self.publish(None);

        let (snap_tx, mut snap_rx) = mpsc::channel::<Result<Bearings, String>>(1);
        let mut schedule = Schedule {
            settle_at: None,
            latest_at: None,
            running: false,
            pending: true,
        };
        loop {
            if schedule.pending && !schedule.running {
                schedule.pending = false;
                schedule.running = true;
                schedule.settle_at = None;
                schedule.latest_at = None;
                self.model.set_running(true);
                self.publish(None);
                let home = self.home.clone();
                let timeout = self.config.snapshot_timeout;
                let tx = snap_tx.clone();
                tokio::spawn(async move {
                    let _ = tx.send(run_snapshot(&home, timeout).await).await;
                });
            }
            let deadline = schedule.deadline();
            tokio::select! {
                Some(signal) = fs_rx.recv() => {
                    if self.apply(signal).await {
                        schedule.changed(Instant::now(), &self.config);
                    }
                }
                Some(Command::Refresh) = cmd_rx.recv() => {
                    schedule.pending = true;
                }
                Some(result) = snap_rx.recv() => {
                    schedule.running = false;
                    self.model.set_running(false);
                    match result {
                        Ok(bearings) => self.model.apply_snapshot(&bearings),
                        Err(error) => self.model.snapshot_failed(error),
                    }
                    self.publish(None);
                }
                () = async {
                    match deadline {
                        Some(at) => tokio::time::sleep_until(at).await,
                        None => std::future::pending().await,
                    }
                } => {
                    schedule.pending = true;
                }
                else => break,
            }
        }
    }
}

async fn run_snapshot(home: &FmHome, timeout: Duration) -> Result<Bearings, String> {
    let out = gateway::run(home, &Entry::BearingsSnapshot, timeout)
        .await
        .map_err(|e| e.to_string())?;
    if out.code != Some(0) {
        let detail = out
            .stderr
            .lines()
            .last()
            .unwrap_or("no error output")
            .trim()
            .to_string();
        return Err(format!(
            "fm-bearings-snapshot.sh failed ({}): {detail}",
            gateway::exit_label(out.code)
        ));
    }
    Bearings::parse(&out.stdout).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_only_direct_state_records() {
        let b = Some(std::ffi::OsStr::new("backlog.md"));
        assert!(
            matches!(classify(Path::new("/h/state/a.status"), b), Some(FsChange::Status(id, _)) if id == "a")
        );
        assert!(
            matches!(classify(Path::new("/h/state/a.meta"), b), Some(FsChange::Meta(id, _)) if id == "a")
        );
        assert!(matches!(
            classify(Path::new("/h/data/backlog.md"), b),
            Some(FsChange::Backlog)
        ));
        assert!(classify(Path::new("/h/state/.hash-x"), b).is_none());
        assert!(classify(Path::new("/h/state/.x.status"), b).is_none());
        assert!(classify(Path::new("/h/state/a.inbox/1.msg"), b).is_none());
        assert!(classify(Path::new("/h/state/sub/a.status"), b).is_none());
        assert!(classify(Path::new("/h/state/a.lock"), b).is_none());
    }

    #[tokio::test]
    async fn read_tail_bounds_and_drops_partial_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.status");
        let filler = "working [at=1]: filler line\n".repeat(4000);
        std::fs::write(&path, format!("{filler}done [at=2]: last")).unwrap();
        let (text, mtime) = read_tail(&path).await.unwrap().unwrap();
        assert!(text.len() as u64 <= MAX_RECORD_BYTES);
        assert!(text.starts_with("working [at=1]"));
        assert!(mtime > 0);
        assert_eq!(status::latest_event(&text).unwrap().word, "done");
        assert!(read_tail(&dir.path().join("gone.status"))
            .await
            .unwrap()
            .is_none());
    }
}
