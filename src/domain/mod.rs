// Core business logic - independent of infrastructure
// Contains Yak model, validation rules, domain operations, and port traits

pub mod event;
pub mod event_format;
pub mod events;
pub mod field;
pub mod ports;
pub mod slug;
pub mod yak;
pub mod yak_map;

pub use event::YakEvent;
pub use field::{
    validate_field_name, validate_field_name_format, CONTEXT_FIELD, ID_FIELD, NAME_FIELD,
    STATE_FIELD,
};
pub use slug::{generate_id, slugify, Name, Slug, YakId};
pub use yak::{validate_state, validate_yak_name, Yak};
pub use yak_map::YakMap;

// Re-exports used only in tests
#[cfg(test)]
pub use events::{
    AddedEvent, ContextUpdatedEvent, FieldUpdatedEvent, MovedEvent, RemovedEvent, RenamedEvent,
    StateUpdatedEvent,
};
