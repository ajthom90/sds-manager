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
