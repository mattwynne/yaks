// Port traits - define interfaces between domain and adapters

pub mod display;
pub mod log;
pub mod storage;
pub mod sync;

pub use display::DisplayPort;
pub use log::LogPort;
pub use storage::StoragePort;
pub use sync::SyncPort;
