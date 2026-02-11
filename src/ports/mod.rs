// Port traits - define interfaces between domain and adapters

pub mod display;
pub mod input;
pub mod log;
pub mod storage;
pub mod sync;

pub use display::DisplayPort;
pub use input::InputPort;
pub use log::LogPort;
pub use storage::StoragePort;
pub use sync::SyncPort;
