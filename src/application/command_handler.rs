// CommandHandler trait - structural enforcement that routing code
// can only dispatch use cases, never access Application internals.

use super::UseCase;
use anyhow::Result;

/// Trait for dispatching use cases.
///
/// `route_command` accepts `&mut impl CommandHandler`, which means
/// routing code physically cannot access adapter references,
/// Application fields, or domain types beyond use case structs.
/// The compiler enforces the architectural rule.
pub trait CommandHandler {
    fn handle(&mut self, use_case: impl UseCase) -> Result<()>;
}
