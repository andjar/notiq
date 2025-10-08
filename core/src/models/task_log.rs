use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Created,
    Completed,
    Uncompleted,
    Deleted,
}

impl TaskStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "created" => Some(TaskStatus::Created),
            "completed" => Some(TaskStatus::Completed),
            "uncompleted" => Some(TaskStatus::Uncompleted),
            "deleted" => Some(TaskStatus::Deleted),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            TaskStatus::Created => "created".to_string(),
            TaskStatus::Completed => "completed".to_string(),
            TaskStatus::Uncompleted => "uncompleted".to_string(),
            TaskStatus::Deleted => "deleted".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskStatusLog {
    pub id: Option<i64>,
    pub node_id: String,
    pub status: TaskStatus,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl TaskStatusLog {
    /// Create a new task status log entry
    pub fn new(
        node_id: String,
        status: TaskStatus,
        old_value: Option<String>,
        new_value: Option<String>,
    ) -> Self {
        Self {
            id: None,
            node_id,
            status,
            old_value,
            new_value,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_log_creation() {
        let log = TaskStatusLog::new(
            "node-1".to_string(),
            TaskStatus::Completed,
            Some("false".to_string()),
            Some("true".to_string()),
        );
        assert_eq!(log.node_id, "node-1");
        assert_eq!(log.status, TaskStatus::Completed);
    }

    #[test]
    fn test_task_status_conversion() {
        assert_eq!(TaskStatus::from_str("created"), Some(TaskStatus::Created));
        assert_eq!(TaskStatus::from_str("COMPLETED"), Some(TaskStatus::Completed));
        assert_eq!(TaskStatus::from_str("invalid"), None);
    }
}

