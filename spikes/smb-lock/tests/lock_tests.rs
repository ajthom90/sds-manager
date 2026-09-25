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
