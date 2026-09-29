# Sub-project 0: Proof-of-Concept Spikes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Answer the four go/no-go questions in spec §4.1 (SQLite + lock file on SMB; Rust core ↔ WinUI 3 on x64/arm64; Typst → PDF/A; WinUI 3 printing) with throwaway code, and record the answers in `docs/spikes/2026-09-25-poc-results.md`.

**Architecture:** A Cargo workspace under `spikes/` with three throwaway crates — `smbstress` (spike A: lock protocol + invariant-checking workload + CLI), `spike-pdf` (spike C: embedded Typst renderer), `spike-ffi` (spike B: a UniFFI library exercising every FFI feature the core API needs) — plus a WinUI 3 app `spikes/winui/SpikeWinUI` (spikes B and D) that runs a scripted FFI self-test and prints PDFs. Nothing under `spikes/` is ever imported by product code; the deliverable is the results document.

**Tech Stack:** Rust 1.94.0 (edition 2024), rusqlite 0.37 (bundled SQLite), typst / typst-layout / typst-pdf 0.15.1, UniFFI =0.31.0 + uniffi-bindgen-cs v0.11.0+v0.31.0, cargo-xwin, .NET 10 + Windows App SDK (WinUI 3, unpackaged), poppler (`pdftotext`, `pdfinfo`), veraPDF.

**Spec:** `docs/superpowers/specs/2026-09-25-sds-core-design.md` (§4.1 defines the spikes and pass criteria; §7.2–§7.3 define the lock and validated-read protocols that spike A implements; §9.1 defines rendering; §3 and §9.3 define the FFI approach).

**Where each task runs:**

| Tasks | Machine |
|---|---|
| 0–6 | This Mac (Apple Silicon) |
| 7–9 | Windows 11 ARM VM with Claude Code installed (development machine); plus a Windows x64 VM on the person's Hyper-V server for native-x64 runs (Task 8 Step 5, Task 9 Step 3) — no tools needed there |
| 10 | Mac + Windows ARM VM + Windows x64 Hyper-V VM (hosts the Windows share) + the person's NAS (Samba), coordinated by the person |
| 11 | Mac |

**Verification status of the code in this plan:** the Rust code and tests in Tasks 1–4 and the Rust half of Task 6 were compiled and passed on this Mac (Rust 1.94.0) while writing the plan; the sample PDF (final and draft) passed veraPDF PDF/A-2b validation; the C# bindings were generated from the `spike_ffi` library and the C# names used in Task 8 were checked against them. The C# code in Tasks 7–9 has **not** been compiled (no Windows toolchain on the Mac); the Windows executor fixes compile errors as they arise and records every deviation in the results document.

## Global Constraints

- All spike code lives under `spikes/` on branch `spike/poc`; `spikes/README.md` states it is throwaway. Product code must never depend on it.
- Rust toolchain pinned to `1.94.0` via `spikes/rust-toolchain.toml`; all crates `edition = "2024"`, `publish = false`.
- SQLite: `journal_mode=DELETE`, `synchronous=FULL`; **WAL is never used** (spec §7.1).
- Write lock (spec §7.2): lock file `<db>.writelock` created with create-exclusive semantics; stale after **30 s** without heartbeat change measured on the waiter's monotonic clock; break by atomic rename to `<db>.writelock.stale-<uuid>`.
- Validated reads (spec §7.3): check lock file + file change counter (header offset 24) before and after every read transaction; retry on change.
- UniFFI pinned `=0.31.0`, generator `uniffi-bindgen-cs` tag `v0.11.0+v0.31.0`.
- Typst crates pinned `0.15.1`; output **PDF/A-2b**; PDF date = revision date for byte-identical output.
- User text is passed to Typst only as JSON data (`sys.inputs`), never spliced into Typst source (spec §9.1).
- Windows targets: **x64 and arm64**; `TargetPlatformMinVersion` `10.0.19041.0` (Windows 10 22H2 floor from spec §2).
- Commit messages end with:
  ```
  Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC
  ```

## Review Focus

1. **A lock holder that is alive but paused past the stale period** (laptop sleep, VM suspend) → it must discover its lock was broken before committing, not write alongside the new holder. Pinned by `paused_holder_learns_its_lock_was_broken` (Task 1) and the `lost_lock_before_commit` counter (Task 3).
2. **An empty or unparseable lock file** (creator crashed between create and write) → treated as stale after the period and broken; never a panic. Pinned by `empty_lock_file_from_crashed_creator_is_broken` (Task 1).
3. **Database paths with spaces and non-ASCII characters** (`\\server\SDS Files\Société\…`) → every tool works with them. Pinned by `four_processes_then_verify` (Task 3) and the SMB runs' path (Task 10).
4. **Product text that looks like Typst markup** (`#`, `$`, `<b>`, `]`, `#set text(…)`) → printed literally. Pinned by `user_text_is_printed_literally_not_interpreted` (Task 4).
5. **Errors crossing the FFI boundary** — a Rust panic, or a C# change listener that throws → a catchable exception or a counted failure, never a crashed app. Pinned by `failing_listener_is_counted_not_fatal` (Task 5) and the self-test checks "Rust panic -> PanicException" / "throwing C# listener" (Task 8).

---

### Task 0: Branch, workspace, and Mac prerequisites

**Files:**
- Create: `spikes/README.md`, `spikes/Cargo.toml`, `spikes/rust-toolchain.toml`, `spikes/.gitignore`

**Interfaces:**
- Produces: a Cargo workspace at `spikes/` with members `smb-lock`, `pdf-render`, `ffi-core` (created in later tasks).

- [ ] **Step 1: Create the branch**

```bash
cd /Users/ajthom90/projects/sds-manager
git checkout -b spike/poc
```

- [ ] **Step 2: Install Mac tools**

```bash
brew install poppler verapdf llvm
rustup toolchain install 1.94.0
rustup target add --toolchain 1.94.0 x86_64-pc-windows-msvc aarch64-pc-windows-msvc
cargo +1.94.0 install --locked cargo-xwin
cargo +1.94.0 install uniffi-bindgen-cs --git https://github.com/NordSecurity/uniffi-bindgen-cs --tag v0.11.0+v0.31.0
```

Expected: `pdftotext -v`, `verapdf --version`, `cargo xwin --version`, `uniffi-bindgen-cs --help` all succeed.

- [ ] **Step 3: Write the workspace files**

`spikes/README.md`:

```markdown
# Proof-of-concept spikes (THROWAWAY)

Code here answers the go/no-go questions in
`docs/superpowers/specs/2026-09-25-sds-core-design.md` §4.1. It is deliberately
minimal and is **not** the start of the product. Do not import, copy, or build on it;
the product core is written fresh from the spec. Findings live in
`docs/spikes/2026-09-25-poc-results.md`.

| Dir | Spike |
|---|---|
| `smb-lock/` | A — SQLite + lock file on SMB (`smbstress` CLI) |
| `ffi-core/` + `winui/` | B — Rust core ↔ WinUI 3 via UniFFI; D — printing |
| `pdf-render/` | C — Typst → PDF/A-2b |
```

`spikes/Cargo.toml` (Tasks 4 and 6 add `pdf-render` and `ffi-core`):

```toml
[workspace]
resolver = "3"
members = ["smb-lock"]
```

`spikes/rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.94.0"
```

`spikes/.gitignore`:

```
target/
out/
pdf-render/assets/fonts/
winui/SpikeWinUI/bin/
winui/SpikeWinUI/obj/
```

- [ ] **Step 4: Commit**

The workspace does not build until Task 1 creates `smb-lock`; commit the scaffolding anyway.

```bash
git add spikes
git commit -m "spikes: scaffold throwaway proof-of-concept workspace

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
```

---

### Task 1: Spike A — write-lock protocol (`smbstress::lock`)

Implements spec §7.2 as a small module: create-exclusive lock file with JSON holder info, jittered backoff, stale detection by unchanged content on the waiter's monotonic clock, rename-based breaking guarded by a `.breaking` file, heartbeat, and `still_held()` for the paused-holder case.

**Files:**
- Create: `spikes/smb-lock/Cargo.toml`, `spikes/smb-lock/src/lib.rs`, `spikes/smb-lock/src/lock.rs`
- Test: `spikes/smb-lock/tests/lock_tests.rs`

**Interfaces:**
- Produces (used by Tasks 2, 3):
  - `pub struct LockInfo { pub client: String, pub machine: String, pub pid: u32, pub session: String, pub heartbeat: u64 }` with `LockInfo::new(client: &str, machine: &str) -> LockInfo`
  - `pub struct LockConfig { pub stale_after: Duration, pub timeout: Duration }` (`Default` = 30 s / 10 s)
  - `pub struct LockStats { pub acquired: u64, pub waits: u64, pub stale_breaks: u64, pub timeouts: u64 }` (`Default`, `Serialize`)
  - `pub enum LockError { Busy(Option<LockInfo>), Io(std::io::Error) }`
  - `pub fn lock_path(db: &Path) -> PathBuf` → `<db>.writelock`
  - `WriteLock::acquire(db: &Path, me: &LockInfo, cfg: &LockConfig, stats: &mut LockStats) -> Result<WriteLock, LockError>`
  - `WriteLock::heartbeat(&mut self) -> io::Result<()>`, `WriteLock::still_held(&self) -> bool`, `WriteLock::release(self) -> io::Result<()>`

- [ ] **Step 1: Create the crate manifest and a lib.rs without the module**

`spikes/smb-lock/Cargo.toml`:

```toml
[package]
name = "smbstress"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
clap = { version = "4", features = ["derive"] }
hex = "0.4"
rand = "0.9"
rusqlite = { version = "0.37", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
tempfile = "3"
```

`spikes/smb-lock/src/lib.rs`:

```rust
//! THROWAWAY spike code (Sub-project 0, spike A). Do not build on this.
```

Also create a placeholder binary so the package builds (it fails every command until Task 3 replaces it): `spikes/smb-lock/src/main.rs`:

```rust
fn main() {
    std::process::exit(2); // replaced in Task 3
}
```

- [ ] **Step 2: Write the failing tests**

`spikes/smb-lock/tests/lock_tests.rs`:

```rust
use smbstress::lock::{lock_path, LockConfig, LockError, LockInfo, LockStats, WriteLock};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

fn cfg(stale_ms: u64, timeout_ms: u64) -> LockConfig {
    LockConfig { stale_after: Duration::from_millis(stale_ms), timeout: Duration::from_millis(timeout_ms) }
}

#[test]
fn second_acquire_is_busy_and_names_holder() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    let a = LockInfo::new("alice", "PC1");
    let _held = WriteLock::acquire(&db, &a, &cfg(60_000, 1_000), &mut LockStats::default()).unwrap();
    let b = LockInfo::new("bob", "PC2");
    match WriteLock::acquire(&db, &b, &cfg(60_000, 300), &mut LockStats::default()) {
        Err(LockError::Busy(Some(holder))) => assert_eq!(holder.client, "alice"),
        other => panic!("expected Busy(alice), got {:?}", other.map(|_| ())),
    }
}

#[test]
fn release_allows_reacquire() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    let me = LockInfo::new("alice", "PC1");
    let l = WriteLock::acquire(&db, &me, &cfg(60_000, 1_000), &mut LockStats::default()).unwrap();
    l.release().unwrap();
    assert!(!lock_path(&db).exists());
    WriteLock::acquire(&db, &me, &cfg(60_000, 1_000), &mut LockStats::default()).unwrap();
}

#[test]
fn unchanged_lock_is_broken_after_stale_period() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    std::fs::write(lock_path(&db), br#"{"client":"dead","machine":"X","pid":1,"session":"s","heartbeat":0}"#).unwrap();
    let mut stats = LockStats::default();
    let start = Instant::now();
    WriteLock::acquire(&db, &LockInfo::new("bob", "PC2"), &cfg(300, 5_000), &mut stats).unwrap();
    assert_eq!(stats.stale_breaks, 1);
    assert!(start.elapsed() >= Duration::from_millis(300));
}

#[test]
fn heartbeat_prevents_stale_break() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    let mut held = WriteLock::acquire(&db, &LockInfo::new("alice", "PC1"), &cfg(60_000, 1_000), &mut LockStats::default()).unwrap();
    let db2 = db.clone();
    let waiter = std::thread::spawn(move || {
        let mut stats = LockStats::default();
        let r = WriteLock::acquire(&db2, &LockInfo::new("bob", "PC2"), &cfg(300, 1_200), &mut stats);
        (r.is_err(), stats.stale_breaks)
    });
    for _ in 0..14 {
        std::thread::sleep(Duration::from_millis(100));
        held.heartbeat().unwrap();
    }
    let (busy, breaks) = waiter.join().unwrap();
    assert!(busy, "waiter must not get the lock while heartbeats continue");
    assert_eq!(breaks, 0);
    assert!(held.still_held());
}

#[test]
fn lock_gives_mutual_exclusion_across_threads() {
    let dir = tempfile::tempdir().unwrap();
    let db = Arc::new(dir.path().join("t.sdsdb"));
    let counter = Arc::new(dir.path().join("counter"));
    std::fs::write(&*counter, "0").unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let (db, counter, barrier) = (db.clone(), counter.clone(), barrier.clone());
            std::thread::spawn(move || {
                let me = LockInfo::new(&format!("t{i}"), "PC");
                barrier.wait();
                for _ in 0..25 {
                    let l = WriteLock::acquire(&db, &me, &cfg(60_000, 30_000), &mut LockStats::default()).unwrap();
                    let n: u64 = std::fs::read_to_string(&*counter).unwrap().parse().unwrap();
                    std::thread::sleep(Duration::from_millis(1));
                    std::fs::write(&*counter, (n + 1).to_string()).unwrap();
                    l.release().unwrap();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(std::fs::read_to_string(&*counter).unwrap(), "200");
}

#[test]
fn empty_lock_file_from_crashed_creator_is_broken() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    std::fs::write(lock_path(&db), b"").unwrap();
    let mut stats = LockStats::default();
    WriteLock::acquire(&db, &LockInfo::new("bob", "PC2"), &cfg(300, 5_000), &mut stats).unwrap();
    assert_eq!(stats.stale_breaks, 1);
}

#[test]
fn paused_holder_learns_its_lock_was_broken() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    // Alice takes the lock, then "sleeps" (no heartbeats) past the stale period.
    let alice = WriteLock::acquire(&db, &LockInfo::new("alice", "LAPTOP"), &cfg(60_000, 1_000), &mut LockStats::default()).unwrap();
    let bob = WriteLock::acquire(&db, &LockInfo::new("bob", "PC2"), &cfg(300, 5_000), &mut LockStats::default()).unwrap();
    assert!(!alice.still_held(), "alice must detect the break before committing");
    assert!(bob.still_held());
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cd spikes && cargo test -p smbstress --test lock_tests`
Expected: compile error `unresolved import smbstress::lock`.

- [ ] **Step 4: Implement the lock module**

`spikes/smb-lock/src/lib.rs`:

```rust
//! THROWAWAY spike code (Sub-project 0, spike A). Do not build on this.
pub mod lock;
```

`spikes/smb-lock/src/lock.rs`:

```rust
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
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd spikes && cargo test -p smbstress --test lock_tests`
Expected: `test result: ok. 7 passed`.

- [ ] **Step 6: Commit**

```bash
git add spikes/Cargo.toml spikes/Cargo.lock spikes/smb-lock
git commit -m "spike A: lock-file write lock with stale detection and breaking

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
```

---

### Task 2: Spike A — invariant workload and validated reads (`smbstress::workload`)

Every write transaction moves an amount between two of 100 accounts (so the total never changes) and writes 1–20 ledger rows of 200–2000 random bytes (multi-page writes) whose SHA-256 is stored in `txn`. A reader that observes a mix of old and new pages breaks the total or a checksum. `validated_read` implements spec §7.3; with `validate = false` it is the sensitivity control.

**Files:**
- Create: `spikes/smb-lock/src/workload.rs`
- Modify: `spikes/smb-lock/src/lib.rs`
- Test: `spikes/smb-lock/tests/workload_tests.rs`

**Interfaces:**
- Consumes: `smbstress::lock::lock_path` (Task 1).
- Produces (used by Task 3):
  - `pub const ACCOUNTS: i64 = 100; pub const START_BALANCE: i64 = 1_000;`
  - `open(db: &Path) -> rusqlite::Result<Connection>`, `init(db: &Path) -> rusqlite::Result<()>`
  - `write_txn(conn: &mut Connection, client: &str, rng: &mut impl Rng) -> rusqlite::Result<String>` (returns txn id)
  - `pub struct Violations { pub bad_total: u64, pub bad_checksum: u64 }` with `any() -> bool`
  - `check_invariants(conn: &mut Connection, sample_txns: usize) -> rusqlite::Result<Violations>` (`0` = check all txns)
  - `change_counter(db: &Path) -> io::Result<u32>`
  - `pub struct ReadStats { pub reads: u64, pub retries: u64, pub gave_up: u64, pub errors: u64 }`
  - `validated_read(conn: &mut Connection, db: &Path, validate: bool, stats: &mut ReadStats) -> Option<Violations>`
  - `integrity_ok(conn: &Connection) -> rusqlite::Result<bool>`, `txn_exists(conn: &Connection, id: &str) -> rusqlite::Result<bool>`

- [ ] **Step 1: Write the failing tests**

`spikes/smb-lock/tests/workload_tests.rs`:

```rust
use smbstress::workload::{self, ReadStats};

#[test]
fn fresh_db_and_writes_satisfy_invariants() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    workload::init(&db).unwrap();
    let mut conn = workload::open(&db).unwrap();
    let mut rng = rand::rng();
    for _ in 0..20 {
        workload::write_txn(&mut conn, "c1", &mut rng).unwrap();
    }
    let v = workload::check_invariants(&mut conn, 0).unwrap();
    assert!(!v.any(), "{v:?}");
    assert!(workload::integrity_ok(&conn).unwrap());
}

#[test]
fn broken_total_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    workload::init(&db).unwrap();
    let mut conn = workload::open(&db).unwrap();
    conn.execute("UPDATE account SET balance = balance + 1 WHERE id = 3", []).unwrap();
    assert_eq!(workload::check_invariants(&mut conn, 0).unwrap().bad_total, 1);
}

#[test]
fn broken_checksum_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    workload::init(&db).unwrap();
    let mut conn = workload::open(&db).unwrap();
    workload::write_txn(&mut conn, "c1", &mut rand::rng()).unwrap();
    conn.execute("UPDATE ledger SET payload = 'tampered' WHERE seq = 0", []).unwrap();
    assert_eq!(workload::check_invariants(&mut conn, 0).unwrap().bad_checksum, 1);
}

#[test]
fn change_counter_moves_on_commit() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    workload::init(&db).unwrap();
    let before = workload::change_counter(&db).unwrap();
    let mut conn = workload::open(&db).unwrap();
    workload::write_txn(&mut conn, "c1", &mut rand::rng()).unwrap();
    assert_ne!(workload::change_counter(&db).unwrap(), before);
}

#[test]
fn validated_read_waits_out_a_held_lock() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    workload::init(&db).unwrap();
    std::fs::write(smbstress::lock::lock_path(&db), b"{}").unwrap();
    let mut conn = workload::open(&db).unwrap();
    let mut stats = ReadStats::default();
    assert!(workload::validated_read(&mut conn, &db, true, &mut stats).is_none());
    assert_eq!(stats.gave_up, 1);
    std::fs::remove_file(smbstress::lock::lock_path(&db)).unwrap();
    assert!(workload::validated_read(&mut conn, &db, true, &mut stats).is_some());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd spikes && cargo test -p smbstress --test workload_tests`
Expected: compile error `unresolved import smbstress::workload` / `could not find workload`.

- [ ] **Step 3: Implement the workload module**

Add to `spikes/smb-lock/src/lib.rs`:

```rust
pub mod workload;
```

`spikes/smb-lock/src/workload.rs`:

```rust
//! Invariant-checking workload for spike A. Every write transaction moves money
//! between two accounts (total is constant) and writes a multi-page batch of
//! ledger rows whose checksum is stored in `txn`. A reader that sees a mix of
//! old and new pages breaks one of these invariants.
use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub const ACCOUNTS: i64 = 100;
pub const START_BALANCE: i64 = 1_000;

pub fn open(db: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db)?;
    conn.busy_timeout(Duration::from_secs(10))?;
    // journal_mode returns a row, so use query_row rather than execute.
    let mode: String = conn.query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))?;
    assert_eq!(mode.to_lowercase(), "delete");
    conn.execute_batch("PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

pub fn init(db: &Path) -> rusqlite::Result<()> {
    let mut conn = open(db)?;
    let tx = conn.transaction()?;
    tx.execute_batch(
        "CREATE TABLE account(id INTEGER PRIMARY KEY, balance INTEGER NOT NULL);
         CREATE TABLE txn(id TEXT PRIMARY KEY, client TEXT NOT NULL, n_rows INTEGER NOT NULL, checksum TEXT NOT NULL);
         CREATE TABLE ledger(txn_id TEXT NOT NULL REFERENCES txn(id), seq INTEGER NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(txn_id, seq));
         CREATE TABLE checkout(entity INTEGER PRIMARY KEY, client TEXT NOT NULL, heartbeat INTEGER NOT NULL);",
    )?;
    for id in 0..ACCOUNTS {
        tx.execute("INSERT INTO account(id, balance) VALUES (?1, ?2)", params![id, START_BALANCE])?;
    }
    tx.commit()
}

fn checksum<'a>(payloads: impl Iterator<Item = &'a str>) -> String {
    let mut h = Sha256::new();
    for p in payloads {
        h.update(p.as_bytes());
        h.update([0u8]);
    }
    hex::encode(h.finalize())
}

/// One write transaction. Caller must hold the app write lock.
pub fn write_txn(conn: &mut Connection, client: &str, rng: &mut impl Rng) -> rusqlite::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let (a, b) = (rng.random_range(0..ACCOUNTS), rng.random_range(0..ACCOUNTS));
    let amount = rng.random_range(1..50);
    let n_rows = rng.random_range(1..=20);
    let payloads: Vec<String> = (0..n_rows)
        .map(|_| {
            let len = rng.random_range(200..2000);
            (0..len).map(|_| rng.random_range(b'a'..=b'z') as char).collect()
        })
        .collect();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute("UPDATE account SET balance = balance - ?1 WHERE id = ?2", params![amount, a])?;
    tx.execute("UPDATE account SET balance = balance + ?1 WHERE id = ?2", params![amount, b])?;
    tx.execute(
        "INSERT INTO txn(id, client, n_rows, checksum) VALUES (?1, ?2, ?3, ?4)",
        params![id, client, n_rows, checksum(payloads.iter().map(String::as_str))],
    )?;
    for (seq, p) in payloads.iter().enumerate() {
        tx.execute("INSERT INTO ledger(txn_id, seq, payload) VALUES (?1, ?2, ?3)", params![id, seq as i64, p])?;
    }
    let entity = rng.random_range(0..20);
    tx.execute(
        "INSERT INTO checkout(entity, client, heartbeat) VALUES (?1, ?2, 1)
         ON CONFLICT(entity) DO UPDATE SET client = excluded.client, heartbeat = heartbeat + 1",
        params![entity, client],
    )?;
    tx.commit()?;
    Ok(id)
}

#[derive(Debug, Default, Clone, Serialize, PartialEq)]
pub struct Violations {
    pub bad_total: u64,
    pub bad_checksum: u64,
}

impl Violations {
    pub fn any(&self) -> bool {
        self.bad_total + self.bad_checksum > 0
    }
}

/// Check both invariants inside one read transaction.
pub fn check_invariants(conn: &mut Connection, sample_txns: usize) -> rusqlite::Result<Violations> {
    let tx = conn.transaction()?;
    let mut v = Violations::default();
    let total: i64 = tx.query_row("SELECT COALESCE(SUM(balance), 0) FROM account", [], |r| r.get(0))?;
    if total != ACCOUNTS * START_BALANCE {
        v.bad_total += 1;
    }
    let sql = if sample_txns == 0 {
        "SELECT id, n_rows, checksum FROM txn".to_string()
    } else {
        format!("SELECT id, n_rows, checksum FROM txn ORDER BY random() LIMIT {sample_txns}")
    };
    let txns: Vec<(String, i64, String)> = {
        let mut stmt = tx.prepare(&sql)?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect::<Result<_, _>>()?
    };
    for (id, n_rows, sum) in txns {
        let mut stmt = tx.prepare_cached("SELECT payload FROM ledger WHERE txn_id = ?1 ORDER BY seq")?;
        let payloads: Vec<String> = stmt.query_map([&id], |r| r.get(0))?.collect::<Result<_, _>>()?;
        if payloads.len() as i64 != n_rows || checksum(payloads.iter().map(String::as_str)) != sum {
            v.bad_checksum += 1;
        }
    }
    tx.commit()?;
    Ok(v)
}

/// SQLite file change counter: 4 big-endian bytes at offset 24, read straight from the file.
pub fn change_counter(db: &Path) -> io::Result<u32> {
    let mut f = File::open(db)?;
    f.seek(SeekFrom::Start(24))?;
    let mut b = [0u8; 4];
    f.read_exact(&mut b)?;
    Ok(u32::from_be_bytes(b))
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct ReadStats {
    pub reads: u64,
    pub retries: u64,
    pub gave_up: u64,
    pub errors: u64,
}

/// Validated read (spec §7.3). With `validate = false` it is a plain read, used as
/// the sensitivity control: it shows whether torn reads happen at all without the protocol.
pub fn validated_read(conn: &mut Connection, db: &Path, validate: bool, stats: &mut ReadStats) -> Option<Violations> {
    let lock = crate::lock::lock_path(db);
    for attempt in 0..5u64 {
        if attempt > 0 {
            stats.retries += 1;
            sleep(Duration::from_millis(50 * attempt));
        }
        if validate && lock.exists() {
            continue;
        }
        let before = if validate { change_counter(db).ok() } else { None };
        let result = check_invariants(conn, 3);
        if validate && (lock.exists() || change_counter(db).ok() != before) {
            continue;
        }
        match result {
            Ok(v) => {
                stats.reads += 1;
                return Some(v);
            }
            Err(_) => {
                stats.errors += 1;
                continue;
            }
        }
    }
    stats.gave_up += 1;
    None
}

pub fn integrity_ok(conn: &Connection) -> rusqlite::Result<bool> {
    let r: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    Ok(r == "ok")
}

pub fn txn_exists(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    Ok(conn.query_row("SELECT 1 FROM txn WHERE id = ?1", [id], |_| Ok(())).optional()?.is_some())
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd spikes && cargo test -p smbstress --test workload_tests`
Expected: `test result: ok. 5 passed`.

- [ ] **Step 5: Commit**

```bash
git add spikes/smb-lock
git commit -m "spike A: invariant workload and validated reads

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
```

---

### Task 3: Spike A — `smbstress` CLI and local multi-process tests

Subcommands: `init`, `run` (randomized workload with the protocol, or without it via `--no-app-lock` / `--no-read-validation` for the sensitivity control), `crash` (take the lock, start a write, `abort()`), `verify` (integrity check, invariants over all txns, and every commit logged by any client exists in the DB).

**Files:**
- Modify: `spikes/smb-lock/src/main.rs` (replace placeholder)
- Test: `spikes/smb-lock/tests/multiprocess.rs`

**Interfaces:**
- Consumes: everything Produced by Tasks 1–2.
- Produces (used by Tasks 6 and 10): binary `smbstress` with
  - `smbstress init --db <path>`
  - `smbstress run --db <path> --client <name> --log-dir <local dir> [--secs 60] [--write-ratio 0.3] [--stale-secs 30] [--no-app-lock] [--no-read-validation]` → prints a JSON `RunReport` (fields `client, machine, secs, writes, write_errors, lost_lock_before_commit, read_violations{bad_total,bad_checksum}, lock{acquired,waits,stale_breaks,timeouts}, read{reads,retries,gave_up,errors}`), writes `<log-dir>/<client>.committed` and `<client>.report.json`; exit 1 if any read violation or write error.
  - `smbstress crash --db <path> --client <name>` → with a 16 KiB page cache, starts a transaction that updates an account and inserts 200 × 2 KB ledger rows (forcing dirty pages to spill into the database file), then aborts — leaving the lock file, a hot journal, and real partial writes for another client to roll back.
  - `smbstress verify --db <path> --log-dirs <dir>...` → prints `{"ok", "integrity_ok", "violations", "logged_commits", "missing_commits"}`; exit 0 only if ok.

- [ ] **Step 1: Write the failing tests**

`spikes/smb-lock/tests/multiprocess.rs`:

```rust
//! Local-disk version of spike A: several OS processes, then crash recovery.
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_smbstress");

fn smbstress(args: &[&str]) -> std::process::Output {
    Command::new(BIN).args(args).output().unwrap()
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn four_processes_then_verify() {
    let dir = tempfile::tempdir().unwrap();
    // Real shares look like "\\server\SDS Files\Société"; spaces and non-ASCII must work.
    let share = dir.path().join("SDS Files").join("Société");
    std::fs::create_dir_all(&share).unwrap();
    let db = share.join("Données test.sdsdb");
    let logs = dir.path().join("logs");
    assert!(smbstress(&["init", "--db", s(&db)]).status.success());
    let children: Vec<_> = (0..4)
        .map(|i| {
            Command::new(BIN)
                .args(["run", "--db", s(&db), "--client", &format!("p{i}"), "--log-dir", s(&logs), "--secs", "10", "--write-ratio", "0.4"])
                .spawn()
                .unwrap()
        })
        .collect();
    for mut c in children {
        assert!(c.wait().unwrap().success(), "a client reported violations or write errors");
    }
    let out = smbstress(&["verify", "--db", s(&db), "--log-dirs", s(&logs)]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn crashed_writer_is_recovered() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.sdsdb");
    let logs = dir.path().join("logs");
    assert!(smbstress(&["init", "--db", s(&db)]).status.success());
    let crash = smbstress(&["crash", "--db", s(&db), "--client", "doomed"]);
    assert!(!crash.status.success());
    assert!(smbstress::lock::lock_path(&db).exists(), "crash must leave the lock file behind");
    let journal = std::path::PathBuf::from(format!("{}-journal", db.display()));
    assert!(journal.exists(), "crash must leave a hot journal behind");
    let size_after_crash = std::fs::metadata(&db).unwrap().len();
    assert!(size_after_crash > 100 * 4096, "dirty pages should have spilled into the db file, size = {size_after_crash}");
    let run = smbstress(&["run", "--db", s(&db), "--client", "survivor", "--log-dir", s(&logs), "--secs", "6", "--stale-secs", "2", "--write-ratio", "0.5"]);
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stdout));
    let report: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(report["lock"]["stale_breaks"], 1);
    let out = smbstress(&["verify", "--db", s(&db), "--log-dirs", s(&logs)]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd spikes && cargo test -p smbstress --test multiprocess`
Expected: both tests FAIL at the first `init` assertion, because the placeholder binary exits with status 2.

- [ ] **Step 3: Implement the CLI**

`spikes/smb-lock/src/main.rs`:

```rust
//! THROWAWAY spike tool: `smbstress` — hammers one SQLite file with the spec §7.2/§7.3
//! protocol from several machines and checks that nothing tears or goes missing.
use clap::{Parser, Subcommand};
use serde::Serialize;
use smbstress::lock::{LockConfig, LockInfo, LockStats, WriteLock};
use smbstress::workload::{self, ReadStats, Violations};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(about = "Spike A: SQLite + lock-file stress test for network shares")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a fresh database.
    Init { #[arg(long)] db: PathBuf },
    /// Run a randomized read/write workload.
    Run {
        #[arg(long)] db: PathBuf,
        #[arg(long)] client: String,
        /// Local (non-share) directory for this client's committed-txn log and report.
        #[arg(long)] log_dir: PathBuf,
        #[arg(long, default_value_t = 60)] secs: u64,
        #[arg(long, default_value_t = 0.3)] write_ratio: f64,
        #[arg(long, default_value_t = 30)] stale_secs: u64,
        /// Sensitivity control: write without the app lock.
        #[arg(long)] no_app_lock: bool,
        /// Sensitivity control: read without change-counter validation.
        #[arg(long)] no_read_validation: bool,
    },
    /// Take the lock, start a write, then abort the process mid-transaction.
    Crash { #[arg(long)] db: PathBuf, #[arg(long)] client: String },
    /// Check integrity, invariants and that every logged commit exists.
    Verify { #[arg(long)] db: PathBuf, /// Directories containing client `*.committed` logs.
        #[arg(long, num_args = 1..)] log_dirs: Vec<PathBuf> },
}

#[derive(Serialize, Default)]
struct RunReport {
    client: String,
    machine: String,
    secs: u64,
    writes: u64,
    write_errors: u64,
    lost_lock_before_commit: u64,
    read_violations: Violations,
    lock: LockStats,
    read: ReadStats,
}

fn machine() -> String {
    std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "unknown".into())
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.cmd {
        Cmd::Init { db } => {
            workload::init(&db).expect("init");
            println!("initialized {}", db.display());
            0
        }
        Cmd::Run { db, client, log_dir, secs, write_ratio, stale_secs, no_app_lock, no_read_validation } => {
            run(db, client, log_dir, secs, write_ratio, stale_secs, no_app_lock, no_read_validation)
        }
        Cmd::Crash { db, client } => {
            let me = LockInfo::new(&client, &machine());
            let _lock = WriteLock::acquire(&db, &me, &LockConfig::default(), &mut LockStats::default()).expect("lock");
            let conn = workload::open(&db).expect("open");
            // A tiny page cache forces SQLite to spill dirty pages into the database file
            // mid-transaction, so the abort leaves real partial writes for another client's
            // hot-journal rollback to undo (not just an untouched file plus a journal).
            conn.execute_batch("PRAGMA cache_size=-16; BEGIN IMMEDIATE; UPDATE account SET balance = balance - 999 WHERE id = 0;").expect("half write");
            let payload = "x".repeat(2048);
            conn.execute("INSERT INTO txn(id, client, n_rows, checksum) VALUES ('crash', ?1, 200, 'none')", [&client]).expect("crash txn row");
            for seq in 0..200 {
                conn.execute("INSERT INTO ledger(txn_id, seq, payload) VALUES ('crash', ?1, ?2)", rusqlite::params![seq, payload]).expect("spill rows");
            }
            eprintln!("aborting with lock held, transaction open and dirty pages spilled");
            std::process::abort();
        }
        Cmd::Verify { db, log_dirs } => verify(db, log_dirs),
    };
    std::process::exit(code);
}

#[allow(clippy::too_many_arguments)]
fn run(db: PathBuf, client: String, log_dir: PathBuf, secs: u64, write_ratio: f64, stale_secs: u64, no_app_lock: bool, no_read_validation: bool) -> i32 {
    fs::create_dir_all(&log_dir).expect("log dir");
    let mut committed = OpenOptions::new().create(true).append(true).open(log_dir.join(format!("{client}.committed"))).expect("log file");
    let mut conn = workload::open(&db).expect("open db");
    let me = LockInfo::new(&client, &machine());
    let cfg = LockConfig { stale_after: Duration::from_secs(stale_secs), timeout: Duration::from_secs(60) };
    let mut rng = rand::rng();
    let mut rep = RunReport { client: client.clone(), machine: machine(), secs, ..Default::default() };
    let start = Instant::now();
    let mut last_report = Instant::now();
    while start.elapsed() < Duration::from_secs(secs) {
        if rand::random::<f64>() < write_ratio {
            let lock = if no_app_lock {
                None
            } else {
                match WriteLock::acquire(&db, &me, &cfg, &mut rep.lock) {
                    Ok(l) => Some(l),
                    Err(e) => {
                        eprintln!("lock error: {e:?}");
                        continue;
                    }
                }
            };
            if lock.as_ref().is_some_and(|l| !l.still_held()) {
                rep.lost_lock_before_commit += 1;
                continue;
            }
            match workload::write_txn(&mut conn, &client, &mut rng) {
                Ok(id) => {
                    rep.writes += 1;
                    writeln!(committed, "{id}").expect("log write");
                }
                Err(e) => {
                    rep.write_errors += 1;
                    eprintln!("write error: {e}");
                }
            }
            if let Some(l) = lock {
                if let Err(e) = l.release() {
                    eprintln!("release error: {e}");
                }
            }
        } else if let Some(v) = workload::validated_read(&mut conn, &db, !no_read_validation, &mut rep.read) {
            rep.read_violations.bad_total += v.bad_total;
            rep.read_violations.bad_checksum += v.bad_checksum;
            if v.any() {
                eprintln!("INVARIANT VIOLATION seen by reader: {v:?}");
            }
        }
        if last_report.elapsed() > Duration::from_secs(60) {
            eprintln!("{}", serde_json::to_string(&rep).unwrap());
            last_report = Instant::now();
        }
    }
    let json = serde_json::to_string_pretty(&rep).unwrap();
    fs::write(log_dir.join(format!("{client}.report.json")), &json).expect("report");
    println!("{json}");
    if rep.read_violations.any() || rep.write_errors > 0 { 1 } else { 0 }
}

fn verify(db: PathBuf, log_dirs: Vec<PathBuf>) -> i32 {
    let mut conn = workload::open(&db).expect("open db");
    let integrity = workload::integrity_ok(&conn).expect("integrity_check");
    let violations = workload::check_invariants(&mut conn, 0).expect("invariants");
    let (mut logged, mut missing) = (0u64, Vec::new());
    for dir in log_dirs {
        for entry in fs::read_dir(&dir).expect("read log dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e == "committed") {
                for line in BufReader::new(fs::File::open(&path).expect("open log")).lines() {
                    let id = line.expect("read line");
                    logged += 1;
                    if !workload::txn_exists(&conn, &id).expect("lookup") {
                        missing.push(id);
                    }
                }
            }
        }
    }
    let ok = integrity && !violations.any() && missing.is_empty();
    println!("{}", serde_json::json!({ "ok": ok, "integrity_ok": integrity, "violations": violations, "logged_commits": logged, "missing_commits": missing }));
    if ok { 0 } else { 1 }
}
```

- [ ] **Step 4: Run all spike A tests**

Run: `cd spikes && cargo test -p smbstress`
Expected: `lock_tests` 7 passed, `multiprocess` 2 passed, `workload_tests` 5 passed.

- [ ] **Step 5: Local sensitivity-control smoke run**

```bash
cd spikes && cargo build --release -p smbstress
D=$(mktemp -d); ./target/release/smbstress init --db "$D/t.sdsdb"
for i in 1 2 3; do ./target/release/smbstress run --db "$D/t.sdsdb" --client c$i --log-dir "$D/logs" --secs 20 --no-app-lock --no-read-validation > "$D/c$i.json" & done; wait
./target/release/smbstress verify --db "$D/t.sdsdb" --log-dirs "$D/logs"
```

Expected on local APFS: `verify` prints `"ok":true` and no client reports `read_violations` (local POSIX locks are reliable, so the data stays consistent even without the protocol). Clients may report a few `write_errors` (SQLite `database is locked` after the 10 s busy timeout, since three writers contend without the app lock) and therefore exit 1 — that is expected here; the pass signal is `verify`. Record the output; it is the baseline for the SMB control runs in Task 10.

- [ ] **Step 6: Commit**

```bash
git add spikes/smb-lock
git commit -m "spike A: smbstress CLI with crash and verify, multi-process tests

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
```

---

### Task 4: Spike C — Typst renderer to PDF/A-2b (`spike-pdf`)

A `World` implementation that serves one template, in-memory files (pictogram SVGs) and in-memory fonts; data goes in as JSON through `sys.inputs.data`. The sample document is Greek with a Bulgarian heading, a pictogram row, a 120-row composition table (multi-page, header repeats), "Page X of Y" footers, an end-of-document marker, and an optional DRAFT watermark.

**Files:**
- Create: `spikes/pdf-render/Cargo.toml`, `spikes/pdf-render/fetch-fonts.sh`, `spikes/pdf-render/assets/sds.typ`, `spikes/pdf-render/assets/ghs02.svg`, `spikes/pdf-render/src/lib.rs`, `spikes/pdf-render/src/sample.rs`, `spikes/pdf-render/src/bin/render-sample.rs`
- Modify: `spikes/Cargo.toml` (add member)
- Test: `spikes/pdf-render/tests/render_tests.rs`

**Interfaces:**
- Produces (used by Tasks 6 and 9):
  - `pub struct RenderInput<'a> { template: &'a str, data: &'a serde_json::Value, files: &'a HashMap<String, Vec<u8>>, fonts: &'a [Vec<u8>], date: (i32, u8, u8), ident: &'a str }`
  - `pub fn render_pdf(input: &RenderInput) -> Result<Vec<u8>, Vec<String>>`
  - `spike_pdf::sample::{load_assets(crate_dir: &Path) -> Assets, sample_data(draft: bool, user_paragraph: &str) -> serde_json::Value}`
  - binary `render-sample` → writes `spikes/pdf-render/out/sample-el.pdf` (4 pages)

- [ ] **Step 1: Manifest, workspace member, fonts, and assets**

`spikes/pdf-render/Cargo.toml`:

```toml
[package]
name = "spike-pdf"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
serde_json = "1"
typst = "0.15.1"
typst-layout = "0.15.1"
typst-pdf = "0.15.1"

[dev-dependencies]
tempfile = "3"
```

Edit `spikes/Cargo.toml`: `members = ["smb-lock", "pdf-render"]`.

`spikes/pdf-render/fetch-fonts.sh` (then `chmod +x` it and run it):

```bash
#!/usr/bin/env bash
# Noto Sans (SIL OFL 1.1) — Latin, Greek and Cyrillic cover every EU language.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p assets/fonts
base=https://github.com/notofonts/notofonts.github.io/raw/main/fonts/NotoSans/hinted/ttf
for f in NotoSans-Regular NotoSans-Bold; do
  curl -sfL -o "assets/fonts/$f.ttf" "$base/$f.ttf"
done
ls -l assets/fonts
```

Expected: two `.ttf` files of ~620 KB each.

`spikes/pdf-render/assets/ghs02.svg` (a stand-in drawn for the spike — **not** the official pictogram, which Sub-project 2 sources from OSHA/CLP):

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect x="15" y="15" width="70" height="70" transform="rotate(45 50 50)" fill="#fff" stroke="#e00" stroke-width="7"/><path d="M50 28 C60 45 66 55 58 70 C54 60 50 62 48 70 C38 60 42 45 50 28 Z" fill="#000"/></svg>
```

`spikes/pdf-render/assets/sds.typ`:

```typst
#let d = json(bytes(sys.inputs.data))
#set document(title: d.product_name, author: d.supplier)
#set text(font: "Noto Sans", size: 9pt, lang: d.lang)
#set page(
  paper: "a4",
  margin: (top: 28mm, bottom: 20mm, x: 18mm),
  header: context [
    #set text(size: 8pt)
    #grid(columns: (1fr, auto), [*#d.product_name* \ #d.labels.sds_number: #d.sds_number], align(right)[#d.labels.version: #d.version \ #d.labels.revision_date: #d.revision_date])
    #line(length: 100%, stroke: 0.5pt)
  ],
  footer: context [
    #set text(size: 8pt)
    #align(center)[#d.labels.page #counter(page).display() #d.labels.of #counter(page).final().first()]
  ],
  background: if d.draft { rotate(-45deg, text(size: 90pt, fill: rgb(220, 0, 0, 40))[DRAFT]) },
)
#for s in d.sections [
  #block(fill: rgb("#e8eef5"), inset: 4pt, width: 100%)[*#s.number #s.title*]
  #for p in s.paragraphs [#p \ ]
  #if "pictograms" in s [
    #stack(dir: ltr, spacing: 4mm, ..s.pictograms.map(p => image("/assets/" + p + ".svg", width: 18mm)))
  ]
  #if "table" in s [
    #table(columns: (2fr, 1fr, 1fr, 3fr), stroke: 0.4pt, table.header(..s.table.header.map(h => [*#h*])), ..s.table.rows.flatten())
  ]
]
#if d.end_marker != none [#align(center)[— #d.end_marker —]]
```

`spikes/pdf-render/src/sample.rs`:

```rust
//! Sample SDS data and asset loading shared by the `render-sample` binary and the tests.
use std::collections::HashMap;
use std::path::Path;

pub struct Assets {
    pub template: String,
    pub files: HashMap<String, Vec<u8>>,
    pub fonts: Vec<Vec<u8>>,
}

/// Load `assets/sds.typ`, `assets/*.svg` and `assets/fonts/*.ttf` from the crate directory.
pub fn load_assets(crate_dir: &Path) -> Assets {
    let assets = crate_dir.join("assets");
    let template = std::fs::read_to_string(assets.join("sds.typ")).expect("assets/sds.typ");
    let mut files = HashMap::new();
    for entry in std::fs::read_dir(&assets).expect("assets dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "svg") {
            let name = path.file_name().unwrap().to_str().unwrap();
            files.insert(format!("assets/{name}"), std::fs::read(&path).unwrap());
        }
    }
    let mut font_paths: Vec<_> = std::fs::read_dir(assets.join("fonts"))
        .expect("assets/fonts missing: run ./fetch-fonts.sh")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "ttf"))
        .collect();
    font_paths.sort();
    let fonts = font_paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
    Assets { template, files, fonts }
}

/// A Greek-language SDS excerpt with a Bulgarian heading and a 120-row composition table (multi-page).
pub fn sample_data(draft: bool, user_paragraph: &str) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = (0..120)
        .map(|i| serde_json::json!([format!("Ингредиент {i}"), "64-17-5", "1–5 %", "Flam. Liq. 2, H225"]))
        .collect();
    serde_json::json!({
        "product_name": "Καθαριστικό X", "supplier": "Acme", "lang": "el",
        "sds_number": "PX-1-EU", "version": "3", "revision_date": "2026-09-25", "draft": draft,
        "labels": {"sds_number": "Αρ. ΔΔΑ", "version": "Έκδοση", "revision_date": "Αναθεώρηση", "page": "Σελίδα", "of": "από"},
        "end_marker": "Τέλος δελτίου δεδομένων ασφαλείας",
        "sections": [
            {"number": "2", "title": "Προσδιορισμός επικινδυνότητας",
             "paragraphs": ["H225 Υγρό και ατμοί πολύ εύφλεκτα.", user_paragraph],
             "pictograms": ["ghs02", "ghs02"]},
            {"number": "3", "title": "Състав/информация за съставките", "paragraphs": [],
             "table": {"header": ["Name", "CAS", "%", "Classification"], "rows": rows}}
        ]
    })
}
```

Create `spikes/pdf-render/src/lib.rs` containing only:

```rust
pub mod sample;
```

- [ ] **Step 2: Write the failing tests**

`spikes/pdf-render/tests/render_tests.rs`:

```rust
use spike_pdf::{render_pdf, sample, RenderInput};
use std::path::Path;
use std::process::Command;

fn render(draft: bool, paragraph: &str) -> Vec<u8> {
    let a = sample::load_assets(Path::new(env!("CARGO_MANIFEST_DIR")));
    let data = sample::sample_data(draft, paragraph);
    let input = RenderInput { template: &a.template, data: &data, files: &a.files, fonts: &a.fonts, date: (2026, 9, 25), ident: "PX-1-EU-v3" };
    render_pdf(&input).expect("render")
}

/// Text of one page (1-based) via poppler's pdftotext.
fn page_text(pdf: &[u8], page: u32) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.pdf");
    std::fs::write(&path, pdf).unwrap();
    let p = page.to_string();
    let out = Command::new("pdftotext").args(["-f", &p, "-l", &p, path.to_str().unwrap(), "-"]).output().expect("pdftotext (brew install poppler)");
    String::from_utf8(out.stdout).unwrap()
}

fn page_count(pdf: &[u8]) -> u32 {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.pdf");
    std::fs::write(&path, pdf).unwrap();
    let out = Command::new("pdfinfo").arg(&path).output().expect("pdfinfo (brew install poppler)");
    let text = String::from_utf8(out.stdout).unwrap();
    let line = text.lines().find(|l| l.starts_with("Pages:")).expect("Pages line");
    line.split_whitespace().last().unwrap().parse().unwrap()
}

#[test]
fn multi_page_with_page_x_of_y_footer() {
    let pdf = render(false, "x");
    let n = page_count(&pdf);
    assert!(n >= 3, "120-row table should span several pages, got {n}");
    assert!(page_text(&pdf, 1).contains(&format!("Σελίδα 1 από {n}")));
    assert!(page_text(&pdf, n).contains(&format!("Σελίδα {n} από {n}")));
    assert!(page_text(&pdf, n).contains("Τέλος δελτίου δεδομένων ασφαλείας"));
}

#[test]
fn greek_and_cyrillic_text_extracts_correctly() {
    let t = page_text(&render(false, "x"), 1);
    assert!(t.contains("Προσδιορισμός επικινδυνότητας"));
    assert!(t.contains("Състав/информация за съставките"));
}

#[test]
fn table_header_repeats_on_following_pages() {
    let t = page_text(&render(false, "x"), 2);
    assert!(t.contains("Classification"), "header row missing on page 2");
    assert!(t.contains("Ингредиент"));
}

#[test]
fn user_text_is_printed_literally_not_interpreted() {
    let evil = "#raw(\"x\") $math$ <b>not markup</b> ] #set text(size: 40pt)";
    let t = page_text(&render(false, evil), 1);
    assert!(t.contains("#raw(\"x\") $math$ <b>not markup</b> ] #set text(size: 40pt)"), "got: {t}");
}

#[test]
fn identical_input_gives_identical_bytes() {
    assert_eq!(render(true, "x"), render(true, "x"));
}

fn upper_ascii_counts(s: &str) -> [usize; 26] {
    let mut c = [0; 26];
    for b in s.bytes().filter(u8::is_ascii_uppercase) {
        c[(b - b'A') as usize] += 1;
    }
    c
}

#[test]
fn draft_watermark_only_on_drafts() {
    // The rotated watermark is extracted as scattered letters, so compare letter counts:
    // a draft page has exactly the letters D, R, A, F, T more than the final page.
    let (draft, fin) = (page_text(&render(true, "x"), 1), page_text(&render(false, "x"), 1));
    let (d, f, w) = (upper_ascii_counts(&draft), upper_ascii_counts(&fin), upper_ascii_counts("DRAFT"));
    for i in 0..26 {
        assert_eq!(d[i], f[i] + w[i], "letter {}", (b'A' + i as u8) as char);
    }
}

#[test]
fn bad_data_returns_error_not_panic() {
    let a = sample::load_assets(Path::new(env!("CARGO_MANIFEST_DIR")));
    let data = serde_json::json!({"product_name": "only this"});
    let input = RenderInput { template: &a.template, data: &data, files: &a.files, fonts: &a.fonts, date: (2026, 9, 25), ident: "x" };
    let err = render_pdf(&input).expect_err("missing fields must fail");
    assert!(!err.is_empty());
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cd spikes && cargo test -p spike-pdf`
Expected: compile error `unresolved imports spike_pdf::render_pdf, spike_pdf::RenderInput`.

- [ ] **Step 4: Implement the renderer and the sample binary**

`spikes/pdf-render/src/lib.rs`:

```rust
//! THROWAWAY spike code (Sub-project 0, spike C): render an SDS-like document
//! from JSON data with embedded Typst, output PDF/A-2b.
use std::collections::HashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, Duration, IntoValue, Smart};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

pub struct RenderInput<'a> {
    /// Typst template source. It reads the data with `json(bytes(sys.inputs.data))`.
    pub template: &'a str,
    pub data: &'a serde_json::Value,
    /// Files the template may load, keyed by path without leading slash, e.g. "assets/ghs02.svg".
    pub files: &'a HashMap<String, Vec<u8>>,
    /// Raw TTF/OTF font files.
    pub fonts: &'a [Vec<u8>],
    /// Revision date (year, month, day), used as PDF creation date for reproducible output.
    pub date: (i32, u8, u8),
    /// Stable document identifier, e.g. "{sds number}-v{version}".
    pub ident: &'a str,
}

struct SdsWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: Source,
    files: HashMap<String, Bytes>,
    today: Datetime,
}

impl World for SdsWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.main.id()
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() { Ok(self.main.clone()) } else { Err(not_found(id)) }
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(id.get().vpath().get_without_slash()).cloned().ok_or_else(|| not_found(id))
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        Some(self.today)
    }
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.get().vpath().get_without_slash().into())
}

/// Render to PDF/A-2b bytes, or return the Typst error messages.
pub fn render_pdf(input: &RenderInput) -> Result<Vec<u8>, Vec<String>> {
    let fonts: Vec<Font> = input.fonts.iter().flat_map(|f| Font::iter(Bytes::new(f.clone()))).collect();
    let mut inputs = Dict::new();
    inputs.insert("data".into(), input.data.to_string().into_value());
    let library = Library::builder().with_inputs(inputs).build();
    let main_id = RootedPath::new(VirtualRoot::Project, VirtualPath::new("/main.typ").expect("valid path")).intern();
    let (y, m, d) = input.date;
    let today = Datetime::from_ymd(y, m, d).ok_or_else(|| vec![format!("invalid date {y}-{m}-{d}")])?;
    let world = SdsWorld {
        library: LazyHash::new(library),
        book: LazyHash::new(FontBook::from_fonts(&fonts)),
        fonts,
        main: Source::new(main_id, input.template.to_string()),
        files: input.files.iter().map(|(k, v)| (k.clone(), Bytes::new(v.clone()))).collect(),
        today,
    };
    let messages = |diags: &[typst::diag::SourceDiagnostic]| diags.iter().map(|d| d.message.to_string()).collect::<Vec<_>>();
    let doc: PagedDocument = typst::compile(&world).output.map_err(|e| messages(&e))?;
    let options = typst_pdf::PdfOptions {
        ident: Smart::Custom(input.ident.to_string()),
        standards: typst_pdf::PdfStandards::new(&[typst_pdf::PdfStandard::A_2b]).map_err(|e| vec![e.message().to_string()])?,
        timestamp: Some(typst_pdf::Timestamp::new_utc(today)),
        ..Default::default()
    };
    typst_pdf::pdf(&doc, &options).map_err(|e| messages(&e))
}

pub mod sample;
```

`spikes/pdf-render/src/bin/render-sample.rs`:

```rust
use spike_pdf::{render_pdf, sample, RenderInput};
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let a = sample::load_assets(dir);
    let data = sample::sample_data(false, "Δοκιμή: #raw(\"x\") $math$ <b>not markup</b> ]");
    let input = RenderInput { template: &a.template, data: &data, files: &a.files, fonts: &a.fonts, date: (2026, 9, 25), ident: "PX-1-EU-v3" };
    match render_pdf(&input) {
        Ok(pdf) => {
            std::fs::create_dir_all(dir.join("out")).unwrap();
            let out = dir.join("out/sample-el.pdf");
            std::fs::write(&out, pdf).unwrap();
            println!("{}", out.display());
        }
        Err(msgs) => {
            eprintln!("render failed: {msgs:?}");
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd spikes && cargo test -p spike-pdf`
Expected: `test result: ok. 7 passed`.

- [ ] **Step 6: Commit**

```bash
git add spikes/Cargo.toml spikes/Cargo.lock spikes/pdf-render
git commit -m "spike C: embedded Typst renderer producing PDF/A-2b

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
```

---

### Task 5: Spike C — PDF/A validation and visual check

**Files:**
- Create: `spikes/pdf-render/validate.sh`

**Interfaces:**
- Consumes: `render-sample` binary (Task 4).
- Produces: `spikes/pdf-render/out/verapdf-report.txt` and `out/page-*.png`, cited in the results document (Task 11).

- [ ] **Step 1: Write the validation script**

`spikes/pdf-render/validate.sh`:

```bash
#!/usr/bin/env bash
# Spike C pass criterion: sample SDS validates as PDF/A-2b and renders correctly.
set -euo pipefail
cd "$(dirname "$0")"
cargo run -q --release --bin render-sample
verapdf --flavour 2b --format text out/sample-el.pdf | tee out/verapdf-report.txt
grep -q "^PASS" out/verapdf-report.txt
pdftoppm -r 60 -png out/sample-el.pdf out/page
pdfinfo out/sample-el.pdf | grep -E "Pages|PDF version|Title"
echo "PDF/A-2b: PASS"
```

- [ ] **Step 2: Run it**

Run: `chmod +x spikes/pdf-render/validate.sh && spikes/pdf-render/validate.sh`
Expected: the veraPDF report line begins with `PASS`, `Pages: 4`, final line `PDF/A-2b: PASS`. If veraPDF reports failures, record every rule ID and message from `out/verapdf-report.txt` in the results document; spike C then fails its criterion and the fallback in spec §4.1 (evaluate `krilla` / `printpdf`) is triggered.

- [ ] **Step 3: Visual check**

Open `spikes/pdf-render/out/page-1.png` and `page-4.png` (use the Read tool on the PNGs). Confirm and note in the results document: header (product name, SDS number, version, revision date) on every page; two pictograms in section 2; table header repeated on page 2 (`page-2.png`); "Σελίδα 4 από 4" footer and the end-of-sheet marker on page 4; Greek and Cyrillic glyphs render (no boxes).

Then re-run with a draft (`sample_data(true, …)` in `render-sample.rs` temporarily), confirm the diagonal red DRAFT watermark is visible and behind the table, and revert the change.

- [ ] **Step 4: Commit**

```bash
git add spikes/pdf-render/validate.sh
git commit -m "spike C: veraPDF validation script

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
```

---

### Task 6: Spike B (Mac half) — UniFFI library, C# bindings, and cross-compiled Windows DLLs

`spike-ffi` exposes one object with a fallible constructor, methods taking and returning records, a typed error enum with fields, a 5 MB byte buffer, a deliberate panic, and a foreign-implemented `ChangeListener` trait called both synchronously and from a Rust background thread. The listener returns `Result<(), ListenerError>` with `From<UnexpectedUniFFICallbackError>`, so an exception thrown in C# arrives in Rust as an `Err` (counted) instead of a panic — this is the pattern the real core's `ChangeListener` (spec §9.3) will use.

**Files:**
- Create: `spikes/ffi-core/Cargo.toml`, `spikes/ffi-core/src/lib.rs`, `spikes/ffi-core/build-windows.sh`
- Create (generated): `spikes/winui/SpikeWinUI/Bindings/spike_ffi.cs`
- Create (build outputs, committed on this throwaway branch so the VM can test Mac-built binaries): `spikes/winui/native-from-mac/win-x64/spike_ffi.dll`, `spikes/winui/native-from-mac/win-arm64/spike_ffi.dll`, `spikes/winui/native-from-mac/win-x64/smbstress.exe`, `spikes/winui/native-from-mac/win-arm64/smbstress.exe`
- Modify: `spikes/Cargo.toml` (add member)
- Test: `spikes/ffi-core/tests/api_tests.rs`

**Interfaces:**
- Produces (C# names verified against generator output while writing this plan; namespace `uniffi.spike_ffi`, all types `internal`):
  - `Session.Open(string path)` (static; throws `SpikeException.Io`), `Session : IDisposable`
  - `session.Save(Item item)` (throws `SpikeException.CheckedOut` with public fields `user`, `machine`)
  - `Item[] session.List()`, `byte[] session.Render(uint size)`, `void session.PanicNow()` (throws `PanicException`)
  - `session.SetListener(ChangeListener listener)`, `session.StartTicker(uint count)`, `uint session.ListenerFailures()`
  - `record Item(string Id, string Name, double Concentration, string[] Tags)`
  - `interface ChangeListener { void OnChange(string[] entityIds); }`

- [ ] **Step 1: Manifest and workspace member**

`spikes/ffi-core/Cargo.toml`:

```toml
[package]
name = "spike-ffi"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
name = "spike_ffi"
crate-type = ["cdylib", "lib"]

[dependencies]
thiserror = "2"
uniffi = "=0.31.0"
```

Edit `spikes/Cargo.toml`: `members = ["smb-lock", "pdf-render", "ffi-core"]`.

Create `spikes/ffi-core/src/lib.rs` containing only:

```rust
uniffi::setup_scaffolding!();
```

- [ ] **Step 2: Write the failing tests**

`spikes/ffi-core/tests/api_tests.rs`:

```rust
use spike_ffi::{ChangeListener, Item, ListenerError, Session, SpikeError};
use std::sync::{Arc, Mutex};

fn item(id: &str, name: &str) -> Item {
    Item { id: id.into(), name: name.into(), concentration: 12.5, tags: vec!["a".into()] }
}

struct Recorder(Mutex<Vec<(String, std::thread::ThreadId)>>);
impl ChangeListener for Recorder {
    fn on_change(&self, ids: Vec<String>) -> Result<(), ListenerError> {
        for id in ids {
            self.0.lock().unwrap().push((id, std::thread::current().id()));
        }
        Ok(())
    }
}

struct Failing;
impl ChangeListener for Failing {
    fn on_change(&self, _: Vec<String>) -> Result<(), ListenerError> {
        Err(ListenerError::Failed { message: "boom".into() })
    }
}

#[test]
fn empty_path_is_io_error() {
    assert!(matches!(Session::open(String::new()), Err(SpikeError::Io { .. })));
}

#[test]
fn save_list_roundtrip_and_checked_out_error() {
    let s = Session::open("x".into()).unwrap();
    s.save(item("1", "Ωμέγα Продукт 漢字")).unwrap();
    assert_eq!(s.list()[0].name, "Ωμέγα Продукт 漢字");
    match s.save(item("2", "locked")) {
        Err(SpikeError::CheckedOut { user, machine }) => assert_eq!((user.as_str(), machine.as_str()), ("jane", "LAB-PC2")),
        other => panic!("expected CheckedOut, got {other:?}"),
    }
}

#[test]
fn failing_listener_is_counted_not_fatal() {
    let s = Session::open("x".into()).unwrap();
    s.set_listener(Arc::new(Failing));
    s.save(item("1", "a")).unwrap();
    s.save(item("2", "b")).unwrap();
    assert_eq!(s.listener_failures(), 2);
    assert_eq!(s.list().len(), 2);
}

#[test]
fn ticker_calls_listener_from_a_background_thread() {
    let s = Session::open("x".into()).unwrap();
    let rec = Arc::new(Recorder(Mutex::new(vec![])));
    s.set_listener(rec.clone());
    s.clone().start_ticker(5);
    std::thread::sleep(std::time::Duration::from_millis(500));
    let calls = rec.0.lock().unwrap();
    assert_eq!(calls.len(), 5);
    assert!(calls.iter().all(|(_, t)| *t != std::thread::current().id()));
}

#[test]
fn render_returns_large_buffers() {
    let s = Session::open("x".into()).unwrap();
    let b = s.render(5_000_000);
    assert_eq!(b.len(), 5_000_000);
    assert_eq!(b[4_999_999], (4_999_999u32 % 251) as u8);
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cd spikes && cargo test -p spike-ffi`
Expected: compile error `unresolved imports spike_ffi::ChangeListener, spike_ffi::Item, ...`.

- [ ] **Step 4: Implement the library**

`spikes/ffi-core/src/lib.rs`:

```rust
//! THROWAWAY spike code (Sub-project 0, spike B): exercises every UniFFI feature the
//! core's API will need, so the C# (and Swift) bindings can be tested end to end.
use std::sync::{Arc, Mutex};
uniffi::setup_scaffolding!();

#[derive(uniffi::Record, Clone, Debug)]
pub struct Item { pub id: String, pub name: String, pub concentration: f64, pub tags: Vec<String> }

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum SpikeError {
    #[error("checked out by {user} on {machine}")]
    CheckedOut { user: String, machine: String },
    #[error("io: {message}")]
    Io { message: String },
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum ListenerError {
    #[error("listener failed: {message}")]
    Failed { message: String },
}

impl From<uniffi::UnexpectedUniFFICallbackError> for ListenerError {
    fn from(e: uniffi::UnexpectedUniFFICallbackError) -> Self { ListenerError::Failed { message: e.reason } }
}

#[uniffi::export(with_foreign)]
pub trait ChangeListener: Send + Sync {
    fn on_change(&self, entity_ids: Vec<String>) -> Result<(), ListenerError>;
}

#[derive(uniffi::Object)]
pub struct Session { items: Mutex<Vec<Item>>, listener: Mutex<Option<Arc<dyn ChangeListener>>>, listener_failures: Mutex<u32> }

impl Session {
    fn notify(&self, ids: Vec<String>) {
        let listener = self.listener.lock().unwrap().clone();
        if let Some(l) = listener {
            if l.on_change(ids).is_err() { *self.listener_failures.lock().unwrap() += 1; }
        }
    }
}

#[uniffi::export]
impl Session {
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, SpikeError> {
        if path.is_empty() { return Err(SpikeError::Io { message: "empty path".into() }); }
        Ok(Arc::new(Self { items: Mutex::new(vec![]), listener: Mutex::new(None), listener_failures: Mutex::new(0) }))
    }
    pub fn set_listener(&self, listener: Arc<dyn ChangeListener>) { *self.listener.lock().unwrap() = Some(listener); }
    pub fn save(&self, item: Item) -> Result<(), SpikeError> {
        if item.name == "locked" { return Err(SpikeError::CheckedOut { user: "jane".into(), machine: "LAB-PC2".into() }); }
        let id = item.id.clone();
        self.items.lock().unwrap().push(item);
        self.notify(vec![id]);
        Ok(())
    }
    pub fn list(&self) -> Vec<Item> { self.items.lock().unwrap().clone() }
    pub fn render(&self, size: u32) -> Vec<u8> { (0..size).map(|i| (i % 251) as u8).collect() }
    pub fn listener_failures(&self) -> u32 { *self.listener_failures.lock().unwrap() }
    pub fn start_ticker(self: Arc<Self>, count: u32) {
        std::thread::spawn(move || for i in 0..count { self.notify(vec![format!("tick-{i}")]); std::thread::sleep(std::time::Duration::from_millis(20)); });
    }
    pub fn panic_now(&self) { panic!("deliberate panic for spike B"); }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd spikes && cargo test -p spike-ffi`
Expected: `test result: ok. 5 passed`.

- [ ] **Step 6: Generate the C# bindings**

```bash
cd spikes
cargo build -p spike-ffi
uniffi-bindgen-cs --library target/debug/libspike_ffi.dylib --out-dir winui/SpikeWinUI/Bindings
grep -n "public static Session Open\|interface ChangeListener\|class CheckedOut\|record Item" winui/SpikeWinUI/Bindings/spike_ffi.cs
```

Expected: `Writing bindings file winui/SpikeWinUI/Bindings/spike_ffi.cs` (a CSharpier formatting warning is harmless) and four grep hits.

- [ ] **Step 7: Cross-compile Windows binaries from the Mac**

`spikes/ffi-core/build-windows.sh`:

```bash
#!/usr/bin/env bash
# Spike B criterion: Windows binaries cross-built on macOS must link and later load on Windows.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$(brew --prefix llvm)/bin:$PATH"
export XWIN_ACCEPT_LICENSE=1   # cargo-xwin downloads the Microsoft CRT/SDK headers
for pair in x86_64-pc-windows-msvc:win-x64 aarch64-pc-windows-msvc:win-arm64; do
  triple=${pair%%:*}; rid=${pair##*:}
  cargo xwin build --release --target "$triple" -p spike-ffi -p smbstress
  mkdir -p "winui/native-from-mac/$rid"
  cp "target/$triple/release/spike_ffi.dll" "target/$triple/release/smbstress.exe" "winui/native-from-mac/$rid/"
done
file winui/native-from-mac/*/*
```

Run: `chmod +x spikes/ffi-core/build-windows.sh && spikes/ffi-core/build-windows.sh`
Expected: `file` reports `PE32+ executable (DLL) ... x86-64` and `... Aarch64` for the DLLs and `PE32+ executable (console)` for both `smbstress.exe`. If the `smbstress` cross-build fails (it compiles SQLite's C code with clang-cl), record the error in the results document and continue: Task 7 builds `smbstress` natively on Windows. Only the `spike_ffi.dll` cross-build is part of the spike B criterion.

- [ ] **Step 8: Commit**

```bash
git add spikes/Cargo.toml spikes/Cargo.lock spikes/ffi-core spikes/winui
git commit -m "spike B: UniFFI test library, C# bindings, Mac-cross-built Windows binaries

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
git push -u origin spike/poc   # the VM clones this branch; create the GitHub remote first if none exists
```

---

### Task 7: Windows VM setup (run on the Windows 11 ARM VM)

**Files:** none (environment only).

**Interfaces:**
- Produces: a VM with Git, Rust 1.94.0 (both MSVC targets), Visual Studio with WinUI/C++ workloads, .NET 10 SDK, Claude Code, the repo on branch `spike/poc`, and native builds of `spike_ffi.dll` and `smbstress.exe` for both architectures.

- [ ] **Step 1: VM**

Create a Windows 11 ARM VM (Parallels Desktop, VMware Fusion, or UTM) with ≥ 8 GB RAM, ≥ 80 GB disk, and **bridged or shared networking such that the Mac can reach the VM's IP and vice versa** (needed for SMB in Task 10). Note the VM's computer name and IP (`hostname`, `ipconfig`).

- [ ] **Step 2: Toolchain (elevated PowerShell)**

```powershell
winget install --id Git.Git -e
winget install --id Rustlang.Rustup -e
winget install --id Microsoft.DotNet.SDK.10 -e
winget install --id Microsoft.VisualStudio.2022.Community -e --override "--quiet --wait --add Microsoft.VisualStudio.Workload.ManagedDesktop --add Microsoft.VisualStudio.Workload.NativeDesktop --add Microsoft.VisualStudio.ComponentGroup.WindowsAppSDK.Cs --add Microsoft.VisualStudio.Component.VC.Tools.ARM64 --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --includeRecommended"
```

If a newer Visual Studio major version is the current release, use its winget id (`winget search Microsoft.VisualStudio`) with the same `--add` components. Open a new terminal, then:

```powershell
rustup toolchain install 1.94.0
rustup target add --toolchain 1.94.0 x86_64-pc-windows-msvc aarch64-pc-windows-msvc
dotnet --version     # expect 10.x
```

Install Claude Code on the VM (per its current Windows install instructions) and sign in.

- [ ] **Step 3: Clone and build natively**

```powershell
git clone https://github.com/ajthom90/sds-manager.git C:\src\sds-manager
cd C:\src\sds-manager
git checkout spike/poc
cd spikes
cargo test -p spike-ffi
cargo test -p smbstress --test lock_tests --test workload_tests
foreach ($t in "aarch64-pc-windows-msvc","x86_64-pc-windows-msvc") { cargo build --release --target $t -p spike-ffi -p smbstress }
```

Expected: tests pass natively on Windows arm64 (this also checks the lock module's Windows error-kind handling on NTFS); four release artifacts under `target\<triple>\release\`. Record any test failure verbatim in the results document before fixing.

---

### Task 8: Spike B (Windows half) — WinUI 3 app calling the Rust core

**Files:**
- Create: `spikes/winui/SpikeWinUI/SpikeWinUI.csproj`, `App.xaml`, `App.xaml.cs`, `MainWindow.xaml`, `MainWindow.xaml.cs`, `SelfTest.cs`, `PdfPrinter.cs` (all under `spikes/winui/SpikeWinUI/`)
- Uses: `spikes/winui/SpikeWinUI/Bindings/spike_ffi.cs` (Task 6)

**Interfaces:**
- Consumes: the C# API listed in Task 6's Interfaces.
- Produces: `SpikeWinUI.exe --selftest <out.json>` → runs all checks, writes `[{ "Name", "Ok", "Detail" }]`, exits; exit code 0 only if all checks are Ok. MSBuild property `NativeSource` = `rust` (default: DLL from `spikes\target\<triple>\release`) or `mac` (DLL from `spikes\winui\native-from-mac\<rid>`). `PdfPrinter` is used by Task 9.

- [ ] **Step 1: Project files**

`spikes/winui/SpikeWinUI/SpikeWinUI.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>WinExe</OutputType>
    <TargetFramework>net10.0-windows10.0.22621.0</TargetFramework>
    <TargetPlatformMinVersion>10.0.19041.0</TargetPlatformMinVersion>
    <RootNamespace>SpikeWinUI</RootNamespace>
    <Platforms>x64;ARM64</Platforms>
    <RuntimeIdentifiers>win-x64;win-arm64</RuntimeIdentifiers>
    <UseWinUI>true</UseWinUI>
    <WindowsPackageType>None</WindowsPackageType>
    <WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>
    <SelfContained>true</SelfContained>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
    <NativeSource Condition="'$(NativeSource)' == ''">rust</NativeSource>
  </PropertyGroup>
  <PropertyGroup Condition="'$(RuntimeIdentifier)' == 'win-arm64'">
    <RustTriple>aarch64-pc-windows-msvc</RustTriple>
  </PropertyGroup>
  <PropertyGroup Condition="'$(RuntimeIdentifier)' == 'win-x64'">
    <RustTriple>x86_64-pc-windows-msvc</RustTriple>
  </PropertyGroup>
  <ItemGroup Condition="'$(NativeSource)' == 'rust'">
    <None Include="..\..\target\$(RustTriple)\release\spike_ffi.dll" Link="spike_ffi.dll" CopyToOutputDirectory="PreserveNewest" />
  </ItemGroup>
  <ItemGroup Condition="'$(NativeSource)' == 'mac'">
    <None Include="..\native-from-mac\$(RuntimeIdentifier)\spike_ffi.dll" Link="spike_ffi.dll" CopyToOutputDirectory="PreserveNewest" />
  </ItemGroup>
</Project>
```

Then add the current stable Windows App SDK packages:

```powershell
cd C:\src\sds-manager\spikes\winui\SpikeWinUI
dotnet add package Microsoft.WindowsAppSDK
dotnet add package Microsoft.Windows.SDK.BuildTools
```

`App.xaml`:

```xml
<Application x:Class="SpikeWinUI.App"
             xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Application.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <XamlControlsResources xmlns="using:Microsoft.UI.Xaml.Controls" />
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </Application.Resources>
</Application>
```

`App.xaml.cs`:

```csharp
using Microsoft.UI.Xaml;

namespace SpikeWinUI;

public partial class App : Application
{
    private Window? _window;

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _window = new MainWindow();
        _window.Activate();
    }
}
```

`MainWindow.xaml`:

```xml
<Window x:Class="SpikeWinUI.MainWindow"
        xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="SDS Manager spikes B + D">
  <Grid Padding="16" RowSpacing="12">
    <Grid.RowDefinitions>
      <RowDefinition Height="Auto" />
      <RowDefinition Height="*" />
      <RowDefinition Height="*" />
    </Grid.RowDefinitions>
    <StackPanel Orientation="Horizontal" Spacing="8">
      <Button Content="Run FFI self-test" Click="SelfTestButton_Click" />
      <Button Content="Print PDF…" Click="PrintButton_Click" />
      <Button Content="Print PDF via WebView2…" Click="WebViewPrintButton_Click" />
    </StackPanel>
    <ScrollViewer Grid.Row="1">
      <TextBlock x:Name="Output" FontFamily="Consolas" TextWrapping="Wrap" IsTextSelectionEnabled="True" />
    </ScrollViewer>
    <WebView2 x:Name="PdfWebView" Grid.Row="2" Visibility="Collapsed" />
  </Grid>
</Window>
```

- [ ] **Step 2: Write the self-test (the failing "test")**

`SelfTest.cs`:

```csharp
using System.Runtime.InteropServices;
using Microsoft.UI.Dispatching;
using uniffi.spike_ffi;

namespace SpikeWinUI;

public record CheckResult(string Name, bool Ok, string Detail);

/// Scripted spike B checks. Each check returns a detail string or throws.
public static class SelfTest
{
    public static async Task<List<CheckResult>> RunAsync(DispatcherQueue ui)
    {
        var results = new List<CheckResult>();
        void Check(string name, Func<string> body)
        {
            try { results.Add(new CheckResult(name, true, body())); }
            catch (Exception e) { results.Add(new CheckResult(name, false, $"{e.GetType().Name}: {e.Message}")); }
        }

        Check("process architecture", () => $"{RuntimeInformation.ProcessArchitecture} process on {RuntimeInformation.OSArchitecture} OS");

        Check("empty path -> SpikeException.Io", () =>
        {
            try { Session.Open(""); }
            catch (SpikeException.Io e) { return e.Message; }
            throw new Exception("no exception thrown");
        });

        using var session = Session.Open("spike");

        Check("record + unicode round trip", () =>
        {
            session.Save(new Item("1", "Ωμέγα Продукт 漢字", 12.5, new[] { "a" }));
            var got = session.List()[0];
            if (got.Name != "Ωμέγα Продукт 漢字" || got.Concentration != 12.5 || got.Tags[0] != "a")
                throw new Exception($"got {got}");
            return got.Name;
        });

        Check("typed error with fields (CheckedOut)", () =>
        {
            try { session.Save(new Item("2", "locked", 0, Array.Empty<string>())); }
            catch (SpikeException.CheckedOut e)
            {
                if (e.user != "jane" || e.machine != "LAB-PC2") throw new Exception($"{e.user}/{e.machine}");
                return $"{e.user} on {e.machine}";
            }
            throw new Exception("no exception thrown");
        });

        Check("5 MB byte[] return", () =>
        {
            var b = session.Render(5_000_000);
            if (b.Length != 5_000_000 || b[4_999_999] != (byte)(4_999_999 % 251)) throw new Exception("bad buffer");
            return $"{b.Length} bytes";
        });

        Check("Rust panic -> PanicException, process survives", () =>
        {
            try { session.PanicNow(); }
            catch (PanicException e) { return e.Message; }
            throw new Exception("no exception thrown");
        });

        Check("session still usable after panic", () => $"{session.List().Length} item(s)");

        var recorder = new RecordingListener();
        session.SetListener(recorder);
        Check("synchronous callback", () =>
        {
            session.Save(new Item("3", "c", 0, Array.Empty<string>()));
            return recorder.Count == 1 ? "1 call" : throw new Exception($"{recorder.Count} calls");
        });

        var tcs = new TaskCompletionSource<string>();
        recorder.Arm(expected: 10, onDone: threads => ui.TryEnqueue(() =>
            tcs.TrySetResult($"10 calls on Rust thread(s) {string.Join(",", threads)}; handled on UI thread {Environment.CurrentManagedThreadId}")));
        session.StartTicker(10);
        var winner = await Task.WhenAny(tcs.Task, Task.Delay(5000));
        results.Add(winner == tcs.Task
            ? new CheckResult("background-thread callbacks marshalled to UI thread", true, tcs.Task.Result)
            : new CheckResult("background-thread callbacks marshalled to UI thread", false, $"timeout after {recorder.Count} calls"));

        session.SetListener(new ThrowingListener());
        Check("throwing C# listener is reported to Rust, not fatal", () =>
        {
            session.Save(new Item("4", "d", 0, Array.Empty<string>()));
            var failures = session.ListenerFailures();
            return failures == 1 ? "listener_failures = 1" : throw new Exception($"listener_failures = {failures}");
        });

        return results;
    }
}

internal sealed class RecordingListener : ChangeListener
{
    private readonly object _gate = new();
    private readonly HashSet<int> _threads = new();
    private int _count;
    private int _expected = int.MaxValue;
    private Action<IEnumerable<int>>? _onDone;

    public int Count { get { lock (_gate) return _count; } }

    public void Arm(int expected, Action<IEnumerable<int>> onDone)
    {
        lock (_gate) { _count = 0; _threads.Clear(); _expected = expected; _onDone = onDone; }
    }

    public void OnChange(string[] entityIds)
    {
        Action<IEnumerable<int>>? done = null;
        int[] threads;
        lock (_gate)
        {
            _count++;
            _threads.Add(Environment.CurrentManagedThreadId);
            threads = _threads.ToArray();
            if (_count == _expected) done = _onDone;
        }
        done?.Invoke(threads);
    }
}

internal sealed class ThrowingListener : ChangeListener
{
    public void OnChange(string[] entityIds) => throw new InvalidOperationException("boom from C#");
}
```

`MainWindow.xaml.cs` (the print buttons are wired to `PdfPrinter`, created in Task 9; until then create `PdfPrinter.cs` with the stub below so the project compiles):

```csharp
using System.Text.Json;
using Microsoft.UI.Xaml;
using Microsoft.Web.WebView2.Core;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace SpikeWinUI;

public sealed partial class MainWindow : Window
{
    private readonly PdfPrinter _printer;
    private bool _selfTestStarted;

    public MainWindow()
    {
        InitializeComponent();
        _printer = new PdfPrinter(WindowNative.GetWindowHandle(this));
        _printer.Completed += c => DispatcherQueue.TryEnqueue(() => Log($"Print task completed: {c}"));

        var args = Environment.GetCommandLineArgs();
        var i = Array.IndexOf(args, "--selftest");
        if (i >= 0 && i + 1 < args.Length)
        {
            var outPath = args[i + 1];
            Activated += async (_, _) =>
            {
                if (_selfTestStarted) return;
                _selfTestStarted = true;
                var results = await RunSelfTestAsync();
                await File.WriteAllTextAsync(outPath, JsonSerializer.Serialize(results, new JsonSerializerOptions { WriteIndented = true }));
                Environment.Exit(results.All(r => r.Ok) ? 0 : 1);
            };
        }
    }

    private void Log(string line) => Output.Text += line + Environment.NewLine;

    private async Task<List<CheckResult>> RunSelfTestAsync()
    {
        var results = await SelfTest.RunAsync(DispatcherQueue);
        foreach (var r in results) Log($"{(r.Ok ? "PASS" : "FAIL")}  {r.Name}: {r.Detail}");
        return results;
    }

    private async void SelfTestButton_Click(object sender, RoutedEventArgs e) => await RunSelfTestAsync();

    private async Task<string?> PickPdfAsync()
    {
        var picker = new FileOpenPicker();
        picker.FileTypeFilter.Add(".pdf");
        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(this));
        var file = await picker.PickSingleFileAsync();
        return file?.Path;
    }

    private async void PrintButton_Click(object sender, RoutedEventArgs e)
    {
        var path = await PickPdfAsync();
        if (path is null) return;
        try { await _printer.PrintAsync(path); Log($"Print UI shown for {path}"); }
        catch (Exception ex) { Log($"PrintManager failed: {ex}"); }
    }

    private async void WebViewPrintButton_Click(object sender, RoutedEventArgs e)
    {
        var path = await PickPdfAsync();
        if (path is null) return;
        PdfWebView.Visibility = Visibility.Visible;
        await PdfWebView.EnsureCoreWebView2Async();
        var loaded = new TaskCompletionSource();
        void Done(CoreWebView2 s, CoreWebView2NavigationCompletedEventArgs a) { s.NavigationCompleted -= Done; loaded.TrySetResult(); }
        PdfWebView.CoreWebView2.NavigationCompleted += Done;
        PdfWebView.CoreWebView2.Navigate(new Uri(path).AbsoluteUri);
        await loaded.Task;
        PdfWebView.CoreWebView2.ShowPrintUI(CoreWebView2PrintDialogKind.System);
        Log($"WebView2 print UI shown for {path}");
    }
}
```

Stub `PdfPrinter.cs` (replaced in Task 9):

```csharp
using Windows.Graphics.Printing;

namespace SpikeWinUI;

public sealed class PdfPrinter
{
    public PdfPrinter(IntPtr hwnd) { }
    public event Action<PrintTaskCompletion>? Completed;
    public Task PrintAsync(string pdfPath) => throw new NotImplementedException("Task 9");
}
```

- [ ] **Step 3: Build and run the self-test on arm64 (native) with the natively built DLL**

```powershell
cd C:\src\sds-manager\spikes\winui\SpikeWinUI
dotnet build -c Release -r win-arm64 -p:Platform=ARM64
$exe = Get-ChildItem -Recurse bin -Filter SpikeWinUI.exe | Where-Object FullName -match "win-arm64" | Select-Object -First 1
$p = Start-Process -FilePath $exe.FullName -ArgumentList "--selftest","$env:TEMP\selftest-arm64-rust.json" -Wait -PassThru; $p.ExitCode
Get-Content "$env:TEMP\selftest-arm64-rust.json"
```

Expected: exit code `0`; every entry `"Ok": true`; "process architecture" says `Arm64 process on Arm64 OS`. Any compile error in the C# above: fix it, and record what changed in the results document.

- [ ] **Step 4: x64 under emulation, and both architectures with the Mac-cross-built DLL**

```powershell
dotnet build -c Release -r win-x64 -p:Platform=x64
$exe = Get-ChildItem -Recurse bin -Filter SpikeWinUI.exe | Where-Object FullName -match "win-x64" | Select-Object -First 1
$p = Start-Process -FilePath $exe.FullName -ArgumentList "--selftest","$env:TEMP\selftest-x64-rust.json" -Wait -PassThru; $p.ExitCode

foreach ($p in @(@{rid="win-arm64";plat="ARM64"}, @{rid="win-x64";plat="x64"})) {
  dotnet build -c Release -r $p.rid -p:Platform=$($p.plat) -p:NativeSource=mac
  $exe = Get-ChildItem -Recurse bin -Filter SpikeWinUI.exe | Where-Object FullName -match $p.rid | Select-Object -First 1
  $proc = Start-Process -FilePath $exe.FullName -ArgumentList "--selftest","$env:TEMP\selftest-$($p.rid)-mac.json" -Wait -PassThru
  "$($p.rid) mac-built: $($proc.ExitCode)"
}
```

Expected: all exit codes `0`; x64 runs report `X64 process on Arm64 OS`. Copy the four JSON files to `spikes/winui/results/` for the record.

- [ ] **Step 5: Native x64 run on the Hyper-V x64 VM**

The build is self-contained (`SelfContained` + `WindowsAppSDKSelfContained`), so the x64 VM needs no SDKs. Copy the whole output folder of each x64 build (the directory containing `SpikeWinUI.exe`, once built with the Rust-on-Windows DLL and once with `-p:NativeSource=mac`) to the x64 VM, e.g. `C:\spike\x64-rust\` and `C:\spike\x64-mac\`. On the x64 VM:

```powershell
foreach ($v in "x64-rust","x64-mac") {
  $p = Start-Process -FilePath "C:\spike\$v\SpikeWinUI.exe" -ArgumentList "--selftest","C:\spike\selftest-$v-native.json" -Wait -PassThru
  "$v native: $($p.ExitCode)"
}
```

Expected: both exit codes `0`; "process architecture" says `X64 process on X64 OS`. If the app fails to start, record the error (Event Viewer → Windows Logs → Application) — a missing runtime dependency here is a packaging finding for Sub-project 5. Copy both JSON files to `spikes/winui/results/`.

- [ ] **Step 6: Commit (on the VM)**

```powershell
git add spikes/winui
git commit -m "spike B: WinUI 3 self-test calling the Rust core on arm64 and x64`n`nCo-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`nClaude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
git push
```

---

### Task 9: Spike D — printing a PDF from WinUI 3

Renders each PDF page to a bitmap with `Windows.Data.Pdf` (at ~300 dpi), then prints through WinUI 3's `PrintDocument` + `PrintManagerInterop`. Fallback: the WebView2 button from Task 8.

**Files:**
- Modify: `spikes/winui/SpikeWinUI/PdfPrinter.cs` (replace stub)

**Interfaces:**
- Consumes: `MainWindow` wiring from Task 8; the sample PDF from Task 4 (`spikes/pdf-render/out/sample-el.pdf`, 4 pages — copy it to the VM, e.g. commit it to `spikes/winui/results/sample-el.pdf`).
- Produces: `PdfPrinter(IntPtr hwnd)`, `Task PrintAsync(string pdfPath)`, `event Action<PrintTaskCompletion> Completed`.

- [ ] **Step 1: Implement PdfPrinter**

`PdfPrinter.cs`:

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Printing;
using Windows.Data.Pdf;
using Windows.Graphics.Printing;
using Windows.Storage;
using Windows.Storage.Streams;

namespace SpikeWinUI;

/// Prints a PDF by rasterising its pages and handing them to the WinUI 3 print pipeline.
public sealed class PdfPrinter
{
    private readonly IntPtr _hwnd;
    private readonly List<Image> _pages = new();
    private PrintDocument? _printDoc;
    private IPrintDocumentSource? _source;
    private string _title = "";

    public event Action<PrintTaskCompletion>? Completed;

    public PdfPrinter(IntPtr hwnd)
    {
        _hwnd = hwnd;
        var manager = PrintManagerInterop.GetForWindow(hwnd);
        manager.PrintTaskRequested += OnPrintTaskRequested;
    }

    public async Task PrintAsync(string pdfPath)
    {
        var file = await StorageFile.GetFileFromPathAsync(pdfPath);
        var pdf = await PdfDocument.LoadFromFileAsync(file);
        _pages.Clear();
        for (uint i = 0; i < pdf.PageCount; i++)
        {
            using var page = pdf.GetPage(i);
            using var stream = new InMemoryRandomAccessStream();
            // Page.Size is in DIPs (1/96 in); render at ~300 dpi for print quality.
            await page.RenderToStreamAsync(stream, new PdfPageRenderOptions { DestinationWidth = (uint)(page.Size.Width * 300 / 96) });
            stream.Seek(0);
            var bitmap = new BitmapImage();
            await bitmap.SetSourceAsync(stream);
            _pages.Add(new Image { Source = bitmap, Stretch = Stretch.Uniform });
        }
        _title = Path.GetFileName(pdfPath);

        _printDoc = new PrintDocument();
        _source = _printDoc.DocumentSource;
        _printDoc.Paginate += (_, e) =>
        {
            var desc = e.PrintTaskOptions.GetPageDescription(0);
            foreach (var img in _pages) { img.Width = desc.PageSize.Width; img.Height = desc.PageSize.Height; }
            _printDoc.SetPreviewPageCount(_pages.Count, PreviewPageCountType.Final);
        };
        _printDoc.GetPreviewPage += (_, e) => _printDoc.SetPreviewPage(e.PageNumber, _pages[e.PageNumber - 1]);
        _printDoc.AddPages += (_, _) =>
        {
            foreach (var img in _pages) _printDoc.AddPage(img);
            _printDoc.AddPagesComplete();
        };

        await PrintManagerInterop.ShowPrintUIForWindowAsync(_hwnd);
    }

    private void OnPrintTaskRequested(PrintManager sender, PrintTaskRequestedEventArgs args)
    {
        var task = args.Request.CreatePrintTask(_title, req => req.SetSource(_source));
        task.Completed += (_, e) => Completed?.Invoke(e.Completion);
    }
}
```

- [ ] **Step 2: Build both architectures**

```powershell
cd C:\src\sds-manager\spikes\winui\SpikeWinUI
dotnet build -c Release -r win-arm64 -p:Platform=ARM64
dotnet build -c Release -r win-x64 -p:Platform=x64
```

Expected: both succeed. Record any compile fixes.

- [ ] **Step 3: Manual print test (the person runs this; the agent records results)**

For each of: the arm64 build and the x64 build on the ARM VM, **and** the x64 build folder copied to the Hyper-V x64 VM (as in Task 8 Step 5):

1. Launch `SpikeWinUI.exe`, click **Print PDF…**, choose `sample-el.pdf`.
2. Pass/fail checks in the print dialog: preview shows **4 pages**; page 1 preview shows the header, two pictograms and the start of the table; no crash or exception text in the app log.
3. Print to **Microsoft Print to PDF**, save as `printed-<arch>.pdf`. The app log must show `Print task completed: Submitted`.
4. Copy the printed files to the Mac and run `pdfinfo printed-<arch>.pdf | grep Pages` (expect `4`) and `pdftoppm -r 60 -png -f 1 -l 1 printed-<arch>.pdf p` — compare with `spikes/pdf-render/out/page-1.png`: nothing cropped at the margins, text sharp at normal zoom.
5. If a physical printer is available, print once and note scaling and quality.
6. Repeat steps 1–4 with **Print PDF via WebView2…** to assess the fallback.

- [ ] **Step 4: Commit**

```powershell
git add spikes/winui
git commit -m "spike D: print PDFs from WinUI 3 via PrintManager, WebView2 fallback`n`nCo-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`nClaude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
git push
```

---

### Task 10: Spike A on real SMB shares

Two server types (spec §4.1): a **Windows-hosted share** on the **Hyper-V x64 VM** (a separate machine, so every client goes over the network) and a **Samba share** on the person's **NAS**. Clients, all against the same file: 2 processes on the Mac (`mac1`, `mac2`), 1 on the Windows ARM VM (`win-arm`, natively built `smbstress.exe`), 1 on the Hyper-V x64 VM (`win-x64`, the Mac-cross-built `spikes\winui\native-from-mac\win-x64\smbstress.exe` — copy it over; this also exercises that binary on real x64). Database path deliberately contains spaces and non-ASCII characters.

**Runtime parameters:** `<vm-user>`, `<vm-ip>`, `<vm-name>`, `<user>`, `<samba-ip>` are the actual account names, addresses and computer names of the machines used (recorded in Task 7 Step 1 and Step 2 below); `<share>` is `winshare` or `samba` for the run in progress. Substitute them when running the commands; everything else is literal.

**Files:**
- Create: `spikes/smb-lock/results/<share>/…` (client reports, verify output — committed for the record)

**Interfaces:**
- Consumes: `smbstress` CLI (Task 3); Windows binaries from Task 6 (`native-from-mac`) or Task 7 (native).

- [ ] **Step 1: Windows-hosted share (elevated PowerShell on the Hyper-V x64 VM)**

```powershell
New-Item -ItemType Directory -Force "C:\sdsspike\spike a" | Out-Null
New-SmbShare -Name sdsspike -Path C:\sdsspike -FullAccess $env:USERNAME
```

On the Mac:

```bash
mkdir -p ~/mnt/winshare
mount_smbfs "//<vm-user>@<vm-ip>/sdsspike" ~/mnt/winshare   # <vm-*> = the Hyper-V x64 VM
```

- [ ] **Step 2: Samba share**

NAS: create a share `sdsspike` with default settings and note its host. Or Ubuntu Server VM (UTM, bridged/shared network):

```bash
sudo apt update && sudo apt install -y samba
sudo mkdir -p "/srv/sdsspike/spike a" && sudo chown -R "$USER" /srv/sdsspike
sudo tee -a /etc/samba/smb.conf <<'EOF'
[sdsspike]
   path = /srv/sdsspike
   read only = no
   browseable = yes
EOF
sudo smbpasswd -a "$USER"
sudo systemctl restart smbd
smbd --version
```

Mac: `mkdir -p ~/mnt/samba && mount_smbfs "//<user>@<samba-ip>/sdsspike" ~/mnt/samba`. Windows: `net use S: \\<samba-ip>\sdsspike /user:<user>`.

Record server OS/versions (Windows build, `smbd --version`) and the negotiated SMB dialect (Mac: `smbutil statshares -a`; Windows: `Get-SmbConnection | Select ServerName,Dialect`).

- [ ] **Step 3: For each share, run the sensitivity control (10 min, no protocol)**

Paths (per share): Mac `DB=~/mnt/winshare/"spike a"/"Société test.sdsdb"` (or `~/mnt/samba/...`); Windows `$db = "\\<vm-name>\sdsspike\spike a\Société test.sdsdb"` with `<vm-name>` = the Hyper-V x64 VM (or `S:\spike a\Société test.sdsdb` for the NAS, after `net use S:` on each Windows VM). Logs go to a **local** directory on each machine. On each Windows VM set `$exe` to that machine's binary: ARM VM `C:\src\sds-manager\spikes\target\aarch64-pc-windows-msvc\release\smbstress.exe`; x64 VM `C:\spike\smbstress.exe` (the copied Mac-cross-built x64 binary).

```bash
# Mac: fresh DB, then two clients
cd /Users/ajthom90/projects/sds-manager/spikes && cargo build --release -p smbstress
rm -f "$DB"; ./target/release/smbstress init --db "$DB"
mkdir -p ~/sds-spike-logs/<share>-control
for c in mac1 mac2; do ./target/release/smbstress run --db "$DB" --client $c --log-dir ~/sds-spike-logs/<share>-control --secs 600 --no-app-lock --no-read-validation > ~/sds-spike-logs/<share>-control/$c.out 2> ~/sds-spike-logs/<share>-control/$c.err & done
```

```powershell
# On each Windows VM, started within a few seconds of the Mac clients; $c = "win-arm" on the ARM VM, "win-x64" on the x64 VM
New-Item -ItemType Directory -Force "C:\sds-spike-logs\<share>-control" | Out-Null
$c = "win-arm"   # use "win-x64" on the x64 VM
Start-Process -NoNewWindow $exe -ArgumentList "run","--db","`"$db`"","--client",$c,"--log-dir","C:\sds-spike-logs\<share>-control","--secs","600","--no-app-lock","--no-read-validation" -RedirectStandardOutput "C:\sds-spike-logs\<share>-control\$c.out" -RedirectStandardError "C:\sds-spike-logs\<share>-control\$c.err"
```

After all four finish, copy both Windows log directories to the Mac and run `smbstress verify --db "$DB" --log-dirs <mac-logs> <windows-logs>`. Record per client: `read_violations`, `write_errors`, and the verify output. **Interpretation:** violations or corruption here show the test is sensitive enough to detect the failure the protocol must prevent; zero violations means the protocol run's clean result is weaker evidence — say so in the results.

- [ ] **Step 4: For each share, the protocol run (60 min) with crash injection**

Same commands as Step 3 (including the `mkdir` / `New-Item` for the new log directories) with a fresh DB, `--secs 3600`, log dir `<share>-protocol`, and **without** the two `--no-…` flags. Then:

- at ~20 min, on the Mac: `./target/release/smbstress crash --db "$DB" --client crash-mac`
- at ~40 min, on the Windows x64 VM: `& $exe crash --db "$db" --client crash-win`

After all clients finish: `smbstress verify` as in Step 3.

**Pass (per share):** all four clients exit 0; `read_violations` all zero; `write_errors` zero; total `stale_breaks` across clients ≥ 2 (one per crash); `verify` prints `"ok":true` with `missing_commits: []`. Also record `lost_lock_before_commit`, `read.retries`, `read.gave_up`, and `lock.timeouts`.

- [ ] **Step 5: Save and commit the evidence**

Copy each run's `*.report.json`, `*.out`, `*.err` and verify output into `spikes/smb-lock/results/<share>-<control|protocol>/` (not the `.committed` logs, which are large).

```bash
git add spikes/smb-lock/results
git commit -m "spike A: SMB run evidence (Windows share and Samba)

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
```

---

### Task 11: Results document and go/no-go decisions

**Files:**
- Create: `docs/spikes/2026-09-25-poc-results.md`
- Modify (only if a spike failed or changed a design assumption): `docs/superpowers/specs/2026-09-25-sds-core-design.md` §3, §4.1, §7, §9, §11

**Interfaces:**
- Consumes: evidence from Tasks 3, 5, 8, 9, 10.

- [ ] **Step 1: Write the results document with this structure, filled with the recorded evidence**

```markdown
# Proof-of-Concept Spike Results

- Date(s) run:
- Branch / commit:
- Spec: docs/superpowers/specs/2026-09-25-sds-core-design.md §4.1

## Summary

| Spike | Criterion met? | Decision |
|---|---|---|
| A. SQLite + lock file on SMB | | Proceed with spec §7 / switch to change-log folder |
| B. Rust core ↔ WinUI 3 | | Proceed with UniFFI + uniffi-bindgen-cs / C ABI fallback |
| C. Typst PDF/A | | Proceed with Typst / evaluate krilla or printpdf |
| D. WinUI 3 printing | | PrintManager / WebView2 fallback |

## A. SQLite + lock file on SMB
Environment (server OS + version, SMB dialect, client OS versions, network), then one table per share
(control run and protocol run): per-client writes, read violations, write errors, stale breaks,
lost-lock-before-commit, read retries / gave-up, lock timeouts; verify output; observations
(e.g. rename/delete errors seen on Windows vs Samba).

## B. Rust core ↔ WinUI 3
Self-test results for arm64-native, x64-emulated, each with Rust-on-Windows and Mac-cross-built DLLs;
cross-compile outcome on the Mac (including smbstress / SQLite); C# changes needed vs the plan.

## C. Typst → PDF/A-2b
veraPDF result, page count, visual check notes, determinism, render time for the sample.
Finding already known: the DRAFT watermark is extracted by text tools as scattered letters;
acceptable for drafts only.

## D. Printing on Windows
PrintManager results per architecture (preview page count, printed page count, scaling, quality),
WebView2 fallback results, recommendation.

## Deviations from the plan
Every change made to plan code or steps, and why.

## Spec changes required
None, or the exact sections to change and how.
```

- [ ] **Step 2: Apply spec changes if any spike failed**

For each failed criterion, edit the spec sections named in its row of spec §4.1 ("If it fails") and §11, following the fallback stated there. If all four passed, write "None" in "Spec changes required".

- [ ] **Step 3: Commit**

```bash
git add docs/spikes docs/superpowers/specs
git commit -m "Record proof-of-concept spike results and decisions

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VbWRc7hWQ841TSUEM8ZMJC"
```

- [ ] **Step 4: Hand back**

Report the summary table to the person. Next step after approval: `superpowers:writing-plans` for Sub-project 1 (the Rust core), using the spike results. (The spike branch itself is not merged into `main`; only `docs/spikes/…` and any spec edits are cherry-picked to `main`.)
