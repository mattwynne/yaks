use crate::domain::event_metadata::Author;

pub trait AuthenticationPort {
    fn current_author(&self) -> Author;
}
