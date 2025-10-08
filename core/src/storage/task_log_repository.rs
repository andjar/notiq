use crate::models::{TaskStatusLog, TaskStatus, datetime_to_timestamp, timestamp_to_datetime};
use crate::{Result};
use rusqlite::{Connection, params};

pub struct TaskLogRepository;

impl TaskLogRepository {
    /// Create a new task log entry
    pub fn create(conn: &Connection, log: &TaskStatusLog) -> Result<i64> {
        conn.execute(
            "INSERT INTO task_status_log (node_id, status, old_value, new_value, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                log.node_id,
                log.status.to_string(),
                log.old_value,
                log.new_value,
                datetime_to_timestamp(&log.timestamp),
            ],
        )?;
        
        Ok(conn.last_insert_rowid())
    }

    /// Get a log entry by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<TaskStatusLog> {
        let mut stmt = conn.prepare(
            "SELECT id, node_id, status, old_value, new_value, timestamp 
             FROM task_status_log WHERE id = ?1"
        )?;
        
        let log = stmt.query_row(params![id], |row| {
            Ok(TaskStatusLog {
                id: Some(row.get(0)?),
                node_id: row.get(1)?,
                status: TaskStatus::from_str(&row.get::<_, String>(2)?)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                old_value: row.get(3)?,
                new_value: row.get(4)?,
                timestamp: timestamp_to_datetime(row.get(5)?),
            })
        })?;
        
        Ok(log)
    }

    /// Get all log entries for a specific node
    pub fn get_by_node_id(conn: &Connection, node_id: &str) -> Result<Vec<TaskStatusLog>> {
        let mut stmt = conn.prepare(
            "SELECT id, node_id, status, old_value, new_value, timestamp 
             FROM task_status_log WHERE node_id = ?1 ORDER BY timestamp DESC"
        )?;
        
        let logs = stmt.query_map(params![node_id], |row| {
            Ok(TaskStatusLog {
                id: Some(row.get(0)?),
                node_id: row.get(1)?,
                status: TaskStatus::from_str(&row.get::<_, String>(2)?)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                old_value: row.get(3)?,
                new_value: row.get(4)?,
                timestamp: timestamp_to_datetime(row.get(5)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(logs)
    }

    /// Get recent task activity (all nodes)
    pub fn get_recent(conn: &Connection, limit: usize) -> Result<Vec<TaskStatusLog>> {
        let mut stmt = conn.prepare(
            "SELECT id, node_id, status, old_value, new_value, timestamp 
             FROM task_status_log ORDER BY timestamp DESC LIMIT ?1"
        )?;
        
        let logs = stmt.query_map(params![limit], |row| {
            Ok(TaskStatusLog {
                id: Some(row.get(0)?),
                node_id: row.get(1)?,
                status: TaskStatus::from_str(&row.get::<_, String>(2)?)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                old_value: row.get(3)?,
                new_value: row.get(4)?,
                timestamp: timestamp_to_datetime(row.get(5)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(logs)
    }

    /// Delete all logs for a specific node
    pub fn delete_by_node_id(conn: &Connection, node_id: &str) -> Result<usize> {
        let rows_affected = conn.execute(
            "DELETE FROM task_status_log WHERE node_id = ?1",
            params![node_id],
        )?;
        
        Ok(rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Note, OutlineNode};
    use crate::storage::{Database, NoteRepository, NodeRepository};
    use tempfile::tempdir;

    fn setup_test_db() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path);
        let conn = db.create().unwrap();
        (dir, conn)
    }

    #[test]
    fn test_create_log() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let node = OutlineNode::new_task(note.id.clone(), None, "Task".to_string(), 0, None, None);
        NodeRepository::create(&conn, &node).unwrap();
        
        let log = TaskStatusLog::new(
            node.id.clone(),
            TaskStatus::Created,
            None,
            Some("true".to_string()),
        );
        
        let id = TaskLogRepository::create(&conn, &log).unwrap();
        assert!(id > 0);
        
        let retrieved = TaskLogRepository::get_by_id(&conn, id).unwrap();
        assert_eq!(retrieved.node_id, node.id);
        assert_eq!(retrieved.status, TaskStatus::Created);
    }

    #[test]
    fn test_get_by_node_id() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let node = OutlineNode::new_task(note.id.clone(), None, "Task".to_string(), 0, None, None);
        NodeRepository::create(&conn, &node).unwrap();
        
        // Create logs with different timestamps
        let mut log1 = TaskStatusLog::new(node.id.clone(), TaskStatus::Created, None, None);
        log1.timestamp = chrono::Utc::now() - chrono::Duration::seconds(10);
        
        let log2 = TaskStatusLog::new(node.id.clone(), TaskStatus::Completed, None, None);
        
        TaskLogRepository::create(&conn, &log1).unwrap();
        TaskLogRepository::create(&conn, &log2).unwrap();
        
        let logs = TaskLogRepository::get_by_node_id(&conn, &node.id).unwrap();
        assert_eq!(logs.len(), 2);
        // Most recent first (Completed should be first since it has a more recent timestamp)
        assert_eq!(logs[0].status, TaskStatus::Completed);
        assert_eq!(logs[1].status, TaskStatus::Created);
    }

    #[test]
    fn test_get_recent() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let node1 = OutlineNode::new_task(note.id.clone(), None, "Task 1".to_string(), 0, None, None);
        let node2 = OutlineNode::new_task(note.id.clone(), None, "Task 2".to_string(), 1, None, None);
        NodeRepository::create(&conn, &node1).unwrap();
        NodeRepository::create(&conn, &node2).unwrap();
        
        let log1 = TaskStatusLog::new(node1.id.clone(), TaskStatus::Created, None, None);
        let log2 = TaskStatusLog::new(node2.id.clone(), TaskStatus::Created, None, None);
        
        TaskLogRepository::create(&conn, &log1).unwrap();
        TaskLogRepository::create(&conn, &log2).unwrap();
        
        let recent = TaskLogRepository::get_recent(&conn, 10).unwrap();
        assert_eq!(recent.len(), 2);
    }
}

