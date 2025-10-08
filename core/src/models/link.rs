use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LinkType {
    Wiki,
    Transclusion,
    Attachment,
}

impl LinkType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "wiki" => Some(LinkType::Wiki),
            "transclusion" => Some(LinkType::Transclusion),
            "attachment" => Some(LinkType::Attachment),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            LinkType::Wiki => "wiki".to_string(),
            LinkType::Transclusion => "transclusion".to_string(),
            LinkType::Attachment => "attachment".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Link {
    pub id: Option<i64>,
    pub source_note_id: String,
    pub source_node_id: Option<String>,
    pub target_note_id: String,
    pub link_text: Option<String>,
    pub link_type: LinkType,
    pub created_at: DateTime<Utc>,
}

impl Link {
    /// Create a new wiki-style link
    pub fn new_wiki_link(
        source_note_id: String,
        source_node_id: Option<String>,
        target_note_id: String,
        link_text: Option<String>,
    ) -> Self {
        Self {
            id: None,
            source_note_id,
            source_node_id,
            target_note_id,
            link_text,
            link_type: LinkType::Wiki,
            created_at: Utc::now(),
        }
    }

    /// Create a new transclusion link
    pub fn new_transclusion(
        source_note_id: String,
        source_node_id: Option<String>,
        target_note_id: String,
        link_text: Option<String>,
    ) -> Self {
        Self {
            id: None,
            source_note_id,
            source_node_id,
            target_note_id,
            link_text,
            link_type: LinkType::Transclusion,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_creation() {
        let link = Link::new_wiki_link(
            "note-1".to_string(),
            Some("node-1".to_string()),
            "note-2".to_string(),
            Some("Target Note".to_string()),
        );
        assert_eq!(link.source_note_id, "note-1");
        assert_eq!(link.target_note_id, "note-2");
        assert_eq!(link.link_type, LinkType::Wiki);
    }

    #[test]
    fn test_link_type_conversion() {
        assert_eq!(LinkType::from_str("wiki"), Some(LinkType::Wiki));
        assert_eq!(LinkType::from_str("TRANSCLUSION"), Some(LinkType::Transclusion));
        assert_eq!(LinkType::from_str("invalid"), None);
    }
}

