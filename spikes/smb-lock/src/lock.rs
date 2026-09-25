//! App-level write lock from spec §7.2: create-exclusive lock file, heartbeat
//! counter, stale detection on the waiter's monotonic clock, break by rename.
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LockInfo {
    pub client: String,
    pub machine: String,
    pub pid: u32,
    pub session: String,
    pub heartbeat: u64,
}

impl LockInfo {
    pub fn new(client: &str, machine: &str) -> Self {
        LockInfo {
            client: client.into(),
            machine: machine.into(),
            pid: std::process::id(),
            session: uuid::Uuid::new_v4().to_string(),
            heartbeat: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LockConfig {
    pub stale_after: Duration,
    pub timeout: Duration,
}

impl Default for LockConfig {
    fn default() -> Self {
        LockConfig { stale_after: Duration::from_secs(30), timeout: Duration::from_secs(10) }
    }
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct LockStats {
    pub acquired: u64,
    pub waits: u64,
    pub stale_breaks: u64,
    pub timeouts: u64,
}

#[derive(Debug)]
pub enum LockError {
    Busy(Option<LockInfo>),
    Io(io::Error),
}

impl From<io::Error> for LockError {
    fn from(e: io::Error) -> Self {
        LockError::Io(e)
    }
}

pub fn lock_path(db: &Path) -> PathBuf {
    let mut s = db.as_os_str().to_owned();
    s.push(".writelock");
    PathBuf::from(s)
}

/// Errors that mean "someone else has (or is deleting) the lock file".
fn is_contention(e: &io::Error) -> bool {
    e.kind() == ErrorKind::AlreadyExists || e.kind() == ErrorKind::PermissionDenied
}

pub struct WriteLock {
    path: PathBuf,
    info: LockInfo,
}

impl WriteLock {
    pub fn acquire(db: &Path, me: &LockInfo, cfg: &LockConfig, stats: &mut LockStats) -> Result<WriteLock, LockError> {
        let path = lock_path(db);
        let start = Instant::now();
        let mut observed: Option<(Vec<u8>, Instant)> = None;
        let mut backoff = Duration::from_millis(50);
        let mut waited = false;
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut f) => {
                    f.write_all(&serde_json::to_vec(me).expect("serialize lock info"))?;
                    f.sync_all()?;
                    stats.acquired += 1;
                    if waited {
                        stats.waits += 1;
                    }
                    return Ok(WriteLock { path, info: me.clone() });
                }
                Err(e) if is_contention(&e) => {}
                Err(e) => return Err(LockError::Io(e)),
            }
            waited = true;
            let content = match fs::read(&path) {
                Ok(c) => c,
                Err(e) if e.kind() == ErrorKind::NotFound => continue, // released meanwhile
                Err(e) if is_contention(&e) => Vec::new(),
                Err(e) => return Err(LockError::Io(e)),
            };
            match &observed {
                Some((c, since)) if *c == content => {
                    if since.elapsed() >= cfg.stale_after {
                        if break_stale(&path, &content)? {
                            stats.stale_breaks += 1;
                        }
                        observed = None;
                        continue;
                    }
                }
                _ => observed = Some((content.clone(), Instant::now())),
            }
            if start.elapsed() >= cfg.timeout {
                stats.timeouts += 1;
                return Err(LockError::Busy(serde_json::from_slice(&content).ok()));
            }
            let jitter = Duration::from_millis(rand::random_range(0..=backoff.as_millis() as u64 / 2));
            sleep(backoff + jitter);
            backoff = (backoff * 2).min(Duration::from_secs(1));
        }
    }

    /// Bump the heartbeat counter so waiters see progress during long operations.
    pub fn heartbeat(&mut self) -> io::Result<()> {
        self.info.heartbeat += 1;
        let mut f = OpenOptions::new().write(true).truncate(true).open(&self.path)?;
        f.write_all(&serde_json::to_vec(&self.info).expect("serialize lock info"))?;
        f.sync_all()
    }

    /// True if the lock file on disk still names this session (not broken by someone else).
    pub fn still_held(&self) -> bool {
        fs::read(&self.path)
            .ok()
            .and_then(|c| serde_json::from_slice::<LockInfo>(&c).ok())
            .is_some_and(|i| i.session == self.info.session)
    }

    pub fn release(self) -> io::Result<()> {
        fs::remove_file(&self.path)
    }
}

/// Rename-then-delete a stale lock. Guarded by a `.breaking` lock so two
/// breakers cannot both act on the same observation. Returns true if this
/// call broke the lock.
fn break_stale(path: &Path, observed: &[u8]) -> io::Result<bool> {
    let mut guard = path.as_os_str().to_owned();
    guard.push(".breaking");
    let guard = PathBuf::from(guard);
    match OpenOptions::new().write(true).create_new(true).open(&guard) {
        Ok(_) => {}
        Err(e) if is_contention(&e) => {
            // Another client is breaking. If its guard is itself abandoned
            // (older than 60 s by file mtime on the server), remove it.
            if let Ok(age) = fs::metadata(&guard).and_then(|m| m.modified()).map(|t| t.elapsed().unwrap_or_default()) {
                if age > Duration::from_secs(60) {
                    let _ = fs::remove_file(&guard);
                }
            }
            return Ok(false);
        }
        Err(e) => return Err(e),
    }
    let result = (|| {
        // Re-check under the guard: only break exactly what we observed.
        match fs::read(path) {
            Ok(c) if c == observed => {}
            Ok(_) => return Ok(false),
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
            Err(e) if is_contention(&e) => return Ok(false),
            Err(e) => return Err(e),
        }
        let mut stale = path.as_os_str().to_owned();
        stale.push(format!(".stale-{}", uuid::Uuid::new_v4()));
        let stale = PathBuf::from(stale);
        match fs::rename(path, &stale) {
            Ok(()) => {
                let _ = fs::remove_file(&stale);
                Ok(true)
            }
            Err(e) if e.kind() == ErrorKind::NotFound || is_contention(&e) => Ok(false),
            Err(e) => Err(e),
        }
    })();
    let _ = fs::remove_file(&guard);
    result
}
