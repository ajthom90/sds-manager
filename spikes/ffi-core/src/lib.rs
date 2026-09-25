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
