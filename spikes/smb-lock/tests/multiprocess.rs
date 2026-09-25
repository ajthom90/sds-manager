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
