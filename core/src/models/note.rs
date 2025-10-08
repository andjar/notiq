use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl Note {
    /// Create a new note with a generated UUID
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            created_at: now,
            modified_at: now,
        }
    }

    /// Create a note with a specific ID (for testing or import)
    pub fn with_id(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            created_at: now,
            modified_at: now,
        }
    }

    /// Update the modified timestamp
    pub fn touch(&mut self) {
        self.modified_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_creation() {
        let note = Note::new("Test Note".to_string());
        assert_eq!(note.title, "Test Note");
        assert!(!note.id.is_empty());
    }

    #[test]
    fn test_note_with_id() {
        let note = Note::with_id("test-id".to_string(), "Test Note".to_string());
        assert_eq!(note.id, "test-id");
        assert_eq!(note.title, "Test Note");
    }

    #[test]
    fn test_note_touch() {
        let mut note = Note::new("Test".to_string());
        let original_modified = note.modified_at;
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        note.touch();
        
        assert!(note.modified_at > original_modified);
    }
}

