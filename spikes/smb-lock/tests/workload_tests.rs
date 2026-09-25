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
