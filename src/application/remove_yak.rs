// Use case: Remove a yak

use crate::adapters::views::Message;
use crate::domain::slug::YakId;
use anyhow::Result;

use super::{Application, UseCase};

pub struct RemoveYak {
    name: String,
    recursive: bool,
}

impl RemoveYak {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            recursive: false,
        }
    }

    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Recursively collect all descendant IDs (breadth-first, parents before children).
    fn collect_descendants(
        parent_id: &YakId,
        all_yaks: &[crate::domain::Yak],
        result: &mut Vec<YakId>,
    ) {
        let children: Vec<&crate::domain::Yak> = all_yaks
            .iter()
            .filter(|yak| yak.parent_id.as_ref() == Some(parent_id))
            .collect();
        for child in children {
            result.push(child.id.clone());
            Self::collect_descendants(&child.id, all_yaks, result);
        }
    }
}

impl UseCase for RemoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.resolve_yak_id(&self.name)?;

        let ids_to_remove = if self.recursive {
            let all_yaks = app.store.list_yaks()?;
            let mut descendants = vec![id.clone()];
            Self::collect_descendants(&id, &all_yaks, &mut descendants);
            for descendant_id in &descendants {
                app.ensure_yak_visible(descendant_id)?;
            }
            // Reverse so leaves come first (children before parents)
            descendants.reverse();
            descendants
        } else {
            vec![id]
        };

        app.with_yak_map(|yak_map| {
            for id in ids_to_remove {
                yak_map.remove_yak(id)?;
            }
            Ok(())
        })?;

        app.display
            .message(&Message::Success(format!("Removed '{}'", self.name)));

        Ok(())
    }
}
