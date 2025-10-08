use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlockType {
    Normal,
    Quote,
    Code,
}

impl TaskPriority {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" => Some(TaskPriority::Low),
            "medium" => Some(TaskPriority::Medium),
            "high" => Some(TaskPriority::High),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            TaskPriority::Low => "low".to_string(),
            TaskPriority::Medium => "medium".to_string(),
            TaskPriority::High => "high".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutlineNode {
    pub id: String,
    pub note_id: String,
    pub parent_node_id: Option<String>,
    pub content: String,
    pub position: i32,
    pub is_task: bool,
    pub task_completed: bool,
    pub task_priority: Option<TaskPriority>,
    pub task_due_date: Option<DateTime<Utc>>,
    pub block_type: BlockType,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

impl OutlineNode {
    /// Create a new outline node
    pub fn new(note_id: String, parent_node_id: Option<String>, content: String, position: i32) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            note_id,
            parent_node_id,
            content,
            position,
            is_task: false,
            task_completed: false,
            task_priority: None,
            task_due_date: None,
            block_type: BlockType::Normal,
            created_at: now,
            modified_at: now,
        }
    }

    /// Create a new task node
    pub fn new_task(
        note_id: String,
        parent_node_id: Option<String>,
        content: String,
        position: i32,
        priority: Option<TaskPriority>,
        due_date: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            note_id,
            parent_node_id,
            content,
            position,
            is_task: true,
            task_completed: false,
            task_priority: priority,
            task_due_date: due_date,
            block_type: BlockType::Normal,
            created_at: now,
            modified_at: now,
        }
    }

    /// Toggle task completion status
    pub fn toggle_task(&mut self) -> bool {
        if self.is_task {
            self.task_completed = !self.task_completed;
            self.touch();
            self.task_completed
        } else {
            false
        }
    }

    /// Update the modified timestamp
    pub fn touch(&mut self) {
        self.modified_at = Utc::now();
    }

    /// Check if this is a root node (no parent)
    pub fn is_root(&self) -> bool {
        self.parent_node_id.is_none()
    }

    /// Create a new special block node (quote or code)
    pub fn new_block(
        note_id: String,
        parent_node_id: Option<String>,
        content: String,
        position: i32,
        block_type: BlockType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            note_id,
            parent_node_id,
            content,
            position,
            is_task: false,
            task_completed: false,
            task_priority: None,
            task_due_date: None,
            block_type,
            created_at: now,
            modified_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outline_node_creation() {
        let node = OutlineNode::new(
            "note-1".to_string(),
            None,
            "Test content".to_string(),
            0,
        );
        assert_eq!(node.content, "Test content");
        assert!(!node.is_task);
        assert!(node.is_root());
    }

    #[test]
    fn test_task_node_creation() {
        let node = OutlineNode::new_task(
            "note-1".to_string(),
            Some("parent-1".to_string()),
            "Task content".to_string(),
            0,
            Some(TaskPriority::High),
            None,
        );
        assert!(node.is_task);
        assert!(!node.task_completed);
        assert_eq!(node.task_priority, Some(TaskPriority::High));
        assert!(!node.is_root());
    }

    #[test]
    fn test_toggle_task() {
        let mut node = OutlineNode::new_task(
            "note-1".to_string(),
            None,
            "Task".to_string(),
            0,
            None,
            None,
        );
        
        assert!(!node.task_completed);
        assert!(node.toggle_task());
        assert!(node.task_completed);
        assert!(!node.toggle_task());
        assert!(!node.task_completed);
    }

    #[test]
    fn test_priority_conversion() {
        assert_eq!(TaskPriority::from_str("low"), Some(TaskPriority::Low));
        assert_eq!(TaskPriority::from_str("HIGH"), Some(TaskPriority::High));
        assert_eq!(TaskPriority::from_str("invalid"), None);
    }
}

