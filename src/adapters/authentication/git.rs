use std::path::Path;

use anyhow::Result;

use crate::domain::event_metadata::Author;
use crate::domain::ports::AuthenticationPort;

pub struct GitAuthentication {
    name: String,
    email: String,
}

impl GitAuthentication {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo = git2::Repository::open(repo_path)?;
        let config = repo.config()?;
        let name = config
            .get_string("user.name")
            .unwrap_or_else(|_| "yx".to_string());
        let email = config
            .get_string("user.email")
            .unwrap_or_else(|_| "yx@localhost".to_string());
        Ok(Self { name, email })
    }
}

impl AuthenticationPort for GitAuthentication {
    fn current_author(&self) -> Author {
        Author {
            name: self.name.clone(),
            email: self.email.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn reads_author_from_git_config() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        let auth = GitAuthentication::new(tmp.path()).unwrap();
        let author = auth.current_author();

        assert_eq!(author.name, "Test User");
        assert_eq!(author.email, "test@example.com");
    }

    #[test]
    fn falls_back_when_git_config_missing() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();

        let auth = GitAuthentication::new(tmp.path()).unwrap();
        let author = auth.current_author();

        // Should not panic, should return some default
        assert!(!author.name.is_empty());
    }
}
