// Use case: Add a new yak

use anyhow::Result;

use super::{Application, UseCase};

/// AddYak use case - creates a new yak
pub struct AddYak {
    name: String,
}

impl AddYak {
    /// Create a new AddYak use case with the yak name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Execute the use case with the application's infrastructure
    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Generate template
        let template = self.generate_context_template()?;

        // Request content via input port
        let context = if let Some(content) = app.input.request_content(None, Some(&template))? {
            if !content.trim().is_empty() {
                Some(content)
            } else {
                None
            }
        } else {
            None
        };

        app.with_yak_map(|yak_map| {
            yak_map.add_yak(self.name.clone(), context)
        })
    }

    fn generate_context_template(&self) -> Result<String> {
        // Parse the yak hierarchy (e.g., "make tea/add milk/go to shops")
        let parts: Vec<&str> = self.name.split('/').collect();

        if parts.len() == 1 {
            // Simple yak, no parents
            return Ok(format!("# {}\n\n", self.name));
        }

        // Nested yak - generate template with parent chain
        let leaf = parts.last().unwrap();
        let mut template = format!("# {}\n\nWhy?\n\n", leaf);

        // Build the parent chain explanation
        for i in 0..parts.len() - 1 {
            let parent_path = parts[0..=i].join("/");
            let parent_name = parts[i];

            if i == 0 {
                template.push_str(&format!(
                    "* We want to *{}* (see `yx context \"{}\"`)\n",
                    parent_name, parent_path
                ));
            } else {
                let prev_parent = parts[i - 1];
                template.push_str(&format!(
                    "* to {}, we need to *{}* (see `yx context \"{}\"`)\n",
                    prev_parent, parent_name, parent_path
                ));
            }
        }

        // Add the final item explaining the current yak
        let last_parent = parts[parts.len() - 2];
        template.push_str(&format!("* to {}, we need to *{}*\n", last_parent, leaf));

        Ok(template)
    }
}

impl UseCase for AddYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage};
    use crate::infrastructure::EventBus;
    use crate::ports::Store;

    #[test]
    fn test_add_yak_creates_yak() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let mut app = Application::new(&mut event_bus, &storage, &display, &input);

        let use_case = AddYak::new("test-yak");
        use_case.execute(&mut app).unwrap();

        assert!(Store::yak_exists(&storage, "test-yak"));
    }

    #[test]
    fn test_generate_context_template_simple_yak() {
        let use_case = AddYak::new("simple-yak");
        let template = use_case.generate_context_template().unwrap();
        assert_eq!(template, "# simple-yak\n\n");
    }

    #[test]
    fn test_generate_context_template_nested_yak() {
        let use_case = AddYak::new("make tea/add milk/go to shops");
        let template = use_case.generate_context_template().unwrap();

        let expected = "# go to shops\n\nWhy?\n\n\
            * We want to *make tea* (see `yx context \"make tea\"`)\n\
            * to make tea, we need to *add milk* (see `yx context \"make tea/add milk\"`)\n\
            * to add milk, we need to *go to shops*\n";

        assert_eq!(template, expected);
    }
}
