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
