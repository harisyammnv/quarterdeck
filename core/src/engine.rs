//! The live engine: keeps a [`FleetView`] current from file events and snapshots.
//!
//! Read path, two speeds:
//! - fast: the home's `state/` directory is watched without recursion; a change to
//!   `<task>.status` or `<task>.meta` re-reads that one small record and publishes a new view
//!   at once, with no debounce, which is what keeps a status change under a second;
//! - true: `fm-fleet-snapshot.sh --json` runs once at start, then once after changes settle
//!   and at most once per `max_interval` while they continue. Only one run is ever in flight,
//!   and only the engine starts one.
//!
//! Every change publishes two things: the [`FleetView`] the fleet list renders, and a
//! [`HomeState`] that each area projects into its own compact view.
//!
//! With a cache directory configured, the last good snapshot is kept on disk (mode 0600, in
//! the app's own data directory) so the window opens with content and reconciles in the
//! background. Nothing here writes into the home.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use crate::fleet::{FleetModel, FleetView, SnapshotInfo, UpdateCause};
use crate::gateway::{self, Entry};
use crate::home::FmHome;
use crate::inbox::{self, NoteError, NoteReceipt};
use crate::snapshot::Snapshot;
use crate::status::StatusEvent;
use crate::{meta, now_ms, status};

/// Largest cached snapshot read back at start, in bytes.
const MAX_CACHE_BYTES: u64 = 32 * 1024 * 1024;

/// Everything a projection may read. Published by the engine on every change.
///
/// Areas build their views from this with a pure `view(&HomeState)` function; it never
/// crosses IPC itself.
#[derive(Clone, Debug, Default)]
pub struct HomeState {
    /// The number of the fleet view published with this state.
    pub seq: u32,
    /// `None` until the first run (or the disk cache) lands.
    pub snapshot: Option<Arc<Snapshot>>,
    /// The latest status event per task id, from the fast path.
    pub events: BTreeMap<String, StatusEvent>,
    pub info: SnapshotInfo,
}

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
    /// Where the last good snapshot is cached, when set. Must be the app's own directory.
    pub cache_dir: Option<PathBuf>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            settle: Duration::from_millis(1500),
            max_interval: Duration::from_secs(10),
            snapshot_timeout: Entry::FleetSnapshot.default_timeout(),
            cache_dir: None,
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

/// A running engine. Dropping the handle stops the watcher and the loop, which closes the
/// view stream for every subscriber.
pub struct EngineHandle {
    home: FmHome,
    views: watch::Receiver<FleetView>,
    states: watch::Receiver<Arc<HomeState>>,
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

    /// The latest home state, for area projections and on-demand readers.
    pub fn state(&self) -> Arc<HomeState> {
        self.states.borrow().clone()
    }

    /// A receiver that wakes on every published home state.
    pub fn subscribe_state(&self) -> watch::Receiver<Arc<HomeState>> {
        self.states.clone()
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
        let (state_tx, state_rx) = watch::channel(Arc::new(home_state(&model)));
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let probes = Arc::new(Mutex::new(Vec::new()));
        let task = Loop {
            home: home.clone(),
            config,
            model,
            views: view_tx,
            states: state_tx,
            probes: probes.clone(),
        };
        tokio::spawn(task.run(fs_rx, cmd_rx));
        Ok(EngineHandle {
            home,
            views: view_rx,
            states: state_rx,
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

/// Reads at most [`MAX_RECORD_BYTES`] of a record. `Ok(None)` when it is gone.
///
/// Records are written by workers, so the read trusts nothing about the path: it opens
/// without following a final symlink and without blocking (a FIFO cannot stall the engine),
/// refuses anything that is not a regular file, and never reads more than the cap.
async fn read_tail(path: &Path) -> std::io::Result<Option<(String, i64)>> {
    let mut file = match tokio::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .await
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let meta = file.metadata().await?;
    if !meta.is_file() {
        return Err(std::io::Error::other("not a regular file"));
    }
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
    // The cap holds even if the file grows between the stat and the read.
    file.take(MAX_RECORD_BYTES).read_to_end(&mut bytes).await?;
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

fn home_state(model: &FleetModel) -> HomeState {
    HomeState {
        seq: model.seq(),
        snapshot: model.snapshot(),
        events: model.events().clone(),
        info: model.info(),
    }
}

/// The cache file for one home: one file per home, so switching homes never shows another
/// home's fleet.
fn cache_file(dir: &Path, home: &FmHome) -> PathBuf {
    // FNV-1a: stable across runs and platforms, unlike the std hasher.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in home.root().as_os_str().as_encoded_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    dir.join(format!("snapshot-{h:016x}.json"))
}

fn read_cache(path: &Path) -> Option<Snapshot> {
    use std::io::Read as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    // Checked on the open handle, not the path, so nothing can be swapped in between.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() > MAX_CACHE_BYTES {
        return None;
    }
    let mut text = String::new();
    file.take(MAX_CACHE_BYTES).read_to_string(&mut text).ok()?;
    Snapshot::parse(&text).ok()
}

/// Writes the cache atomically with mode 0600: it holds the fleet's records.
fn write_cache(path: &Path, json: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::os::unix::fs::PermissionsExt as _;
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    // A unique name per write, so two overlapping writes never share a temporary file.
    static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = dir.join(format!(".snapshot.{}.{n}.tmp", std::process::id()));
    // A leftover from a crashed write is removed, then the file is created exclusively, so a
    // planted link at that name is never followed.
    match std::fs::remove_file(&tmp) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)?;
    f.write_all(json.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

struct Loop {
    home: FmHome,
    config: EngineConfig,
    model: FleetModel,
    views: watch::Sender<FleetView>,
    states: watch::Sender<Arc<HomeState>>,
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
        self.states.send_replace(Arc::new(home_state(&self.model)));
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
        let cache = self
            .config
            .cache_dir
            .as_deref()
            .map(|d| cache_file(d, &self.home));
        if let Some(path) = cache.clone() {
            if let Ok(Some(snap)) = tokio::task::spawn_blocking(move || read_cache(&path)).await {
                self.model.apply_cached(Arc::new(snap));
            }
        }
        self.publish(None);

        let (snap_tx, mut snap_rx) = mpsc::channel::<Result<(Snapshot, String), String>>(1);
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
                command = cmd_rx.recv() => match command {
                    Some(Command::Refresh) => schedule.pending = true,
                    // The handle was dropped (the home changed or the app is closing): stop,
                    // which drops the view sender and ends every subscriber's stream.
                    None => break,
                },
                Some(result) = snap_rx.recv() => {
                    schedule.running = false;
                    self.model.set_running(false);
                    match result {
                        Ok((snapshot, json)) => {
                            self.model.apply_snapshot(Arc::new(snapshot));
                            if let Some(path) = cache.clone() {
                                // Best effort: a cache that cannot be written only costs the
                                // next start its instant first paint.
                                tokio::task::spawn_blocking(move || write_cache(&path, &json));
                            }
                        }
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

async fn run_snapshot(home: &FmHome, timeout: Duration) -> Result<(Snapshot, String), String> {
    let out = gateway::run(home, &Entry::FleetSnapshot, timeout)
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
            "fm-fleet-snapshot.sh failed ({}): {detail}",
            gateway::exit_label(out.code)
        ));
    }
    let snapshot = Snapshot::parse(&out.stdout).map_err(|e| e.to_string())?;
    Ok((snapshot, out.stdout))
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

    #[test]
    fn cache_round_trips_with_private_mode_and_one_file_per_home() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let a = FmHome::at_unchecked(PathBuf::from("/p/home-a"));
        let b = FmHome::at_unchecked(PathBuf::from("/p/home-b"));
        assert_ne!(cache_file(dir.path(), &a), cache_file(dir.path(), &b));
        let path = cache_file(&dir.path().join("nested"), &a);
        let json = include_str!("../tests/fixtures/fleet-snapshot.json");
        write_cache(&path, json).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let dir_mode = std::fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        assert_eq!(read_cache(&path).unwrap().tasks.len(), 6);
        std::fs::write(&path, "{").unwrap();
        assert!(read_cache(&path).is_none(), "a damaged cache is ignored");
        assert!(read_cache(&dir.path().join("absent.json")).is_none());
    }

    #[tokio::test]
    async fn read_tail_refuses_links_and_fifos() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside.txt");
        std::fs::write(&outside, "working [at=1]: secret").unwrap();
        let link = dir.path().join("a.status");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(
            read_tail(&link).await.is_err(),
            "a symlinked record is not followed"
        );
        let fifo = dir.path().join("b.status");
        assert!(std::process::Command::new("/usr/bin/mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success());
        let r = tokio::time::timeout(Duration::from_secs(2), read_tail(&fifo))
            .await
            .expect("a FIFO record must not stall the engine");
        assert!(r.is_err());
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
