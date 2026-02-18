pub mod event_bus;
pub mod git_discovery;
pub use event_bus::EventBus;
pub use git_discovery::{check_yaks_gitignored, discover_git_root};
