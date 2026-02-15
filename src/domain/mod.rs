// Core business logic - independent of infrastructure
// Contains Yak model, validation rules, domain operations, and port traits

pub mod event;
pub mod event_format;
pub mod events;
pub mod field;
pub mod hierarchy;
pub mod ports;
pub mod slug;
pub mod yak;
pub mod yak_map;

pub use event::YakEvent;
pub use field::{validate_field_name, CONTEXT_FIELD, STATE_FIELD};
pub use hierarchy::{find_children, get_ancestors};
pub use slug::generate_slug;
pub use yak::{validate_state, validate_yak_name, Yak};
pub use yak_map::YakMap;

// Re-exports used only in tests
#[cfg(test)]
pub use events::{
    AddedEvent, ContextUpdatedEvent, FieldUpdatedEvent, MovedEvent, RemovedEvent, StateUpdatedEvent,
};
