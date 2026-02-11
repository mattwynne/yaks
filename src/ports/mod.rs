// Port traits - define interfaces between domain and adapters

pub mod display;
pub mod event_listener;
pub mod input;
pub mod log;
pub mod storage;
pub mod store;
pub mod sync;

pub use display::DisplayPort;
pub use event_listener::EventListener;
pub use input::InputPort;
pub use log::LogPort;
pub use storage::StoragePort;
pub use store::Store;
pub use sync::SyncPort;
