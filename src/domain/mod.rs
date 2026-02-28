// Core business logic - independent of infrastructure
// Contains YakView model, validation rules, domain operations, and port traits

pub mod event;
pub mod event_format;
pub mod event_metadata;
pub mod events;
pub mod field;
pub mod ports;
pub mod slug;
pub mod tag;
pub mod yak;
pub mod yak_map;
pub mod yak_snapshot;

pub use event::YakEvent;
pub use event_metadata::{Author, EventMetadata, Timestamp};
pub use field::{
    validate_field_name, validate_field_name_format, CONTEXT_FIELD, CREATED_FIELD, ID_FIELD,
    NAME_FIELD, STATE_FIELD,
};
pub use slug::{generate_id, slugify, Name, Slug, YakId};
pub use tag::{format_tag, normalize_tag};
pub use yak::{validate_state, validate_yak_name, YakView};
pub use yak_map::YakMap;
pub use yak_snapshot::YakSnapshot;

// Re-exports used only in tests
#[cfg(test)]
pub use events::{AddedEvent, FieldUpdatedEvent, MovedEvent, RemovedEvent};
