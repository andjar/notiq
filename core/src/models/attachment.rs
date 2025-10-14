use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attachment {
    pub id: String,
    pub note_id: String,
    pub node_id: String,
    pub filename: String,
    pub filepath: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub hash: String,
    pub created_at: DateTime<Utc>,
}

impl Attachment {
    /// Create a new attachment
    pub fn new(
        note_id: String,
        node_id: String,
        filename: String,
        filepath: String,
        mime_type: Option<String>,
        size_bytes: i64,
        hash: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            note_id,
            node_id,
            filename,
            filepath,
            mime_type,
            size_bytes,
            hash,
            created_at: Utc::now(),
        }
    }

    /// Get human-readable file size
    pub fn human_readable_size(&self) -> String {
        let bytes = self.size_bytes as f64;
        if bytes < 1024.0 {
            format!("{} B", bytes)
        } else if bytes < 1024.0 * 1024.0 {
            format!("{:.1} KB", bytes / 1024.0)
        } else if bytes < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", bytes / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", bytes / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_creation() {
        let attachment = Attachment::new(
            "note-1".to_string(),
            "node-1".to_string(),
            "document.pdf".to_string(),
            "/path/to/document.pdf".to_string(),
            Some("application/pdf".to_string()),
            1024,
            "abc123".to_string(),
        );
        assert_eq!(attachment.filename, "document.pdf");
        assert_eq!(attachment.size_bytes, 1024);
    }

    #[test]
    fn test_human_readable_size() {
        let attachment = Attachment::new(
            "note-1".to_string(),
            "node-1".to_string(),
            "file.txt".to_string(),
            "/path/file.txt".to_string(),
            None,
            1536,
            "hash".to_string(),
        );
        assert_eq!(attachment.human_readable_size(), "1.5 KB");
        
        let large = Attachment::new(
            "note-1".to_string(),
            "node-1".to_string(),
            "large.zip".to_string(),
            "/path/large.zip".to_string(),
            None,
            5_242_880,
            "hash".to_string(),
        );
        assert_eq!(large.human_readable_size(), "5.0 MB");
    }
}

