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
