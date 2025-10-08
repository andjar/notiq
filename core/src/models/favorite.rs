use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Favorite {
    pub note_id: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

impl Favorite {
    /// Create a new favorite
    pub fn new(note_id: String, position: i32) -> Self {
        Self {
            note_id,
            position,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_favorite_creation() {
        let favorite = Favorite::new("note-1".to_string(), 0);
        assert_eq!(favorite.note_id, "note-1");
        assert_eq!(favorite.position, 0);
    }
}

