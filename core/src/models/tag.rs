use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Tag {
    /// Create a new tag
    pub fn new(name: String, color: Option<String>) -> Self {
        Self {
            id: None,
            name,
            color,
            created_at: Utc::now(),
        }
    }

    /// Normalize tag name (lowercase, trim whitespace)
    pub fn normalize_name(name: &str) -> String {
        name.trim().to_lowercase()
    }

    /// Validate tag name
    pub fn is_valid_name(name: &str) -> bool {
        let trimmed = name.trim();
        !trimmed.is_empty() && trimmed.len() <= 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_creation() {
        let tag = Tag::new("work".to_string(), Some("#FF5733".to_string()));
        assert_eq!(tag.name, "work");
        assert_eq!(tag.color, Some("#FF5733".to_string()));
        assert!(tag.id.is_none());
    }

    #[test]
    fn test_normalize_name() {
        assert_eq!(Tag::normalize_name("  Work  "), "work");
        assert_eq!(Tag::normalize_name("ProJect"), "project");
    }

    #[test]
    fn test_is_valid_name() {
        assert!(Tag::is_valid_name("work"));
        assert!(Tag::is_valid_name("  work  "));
        assert!(!Tag::is_valid_name(""));
        assert!(!Tag::is_valid_name("   "));
    }
}

