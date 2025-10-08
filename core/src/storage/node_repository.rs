use crate::models::{OutlineNode, TaskPriority, BlockType, datetime_to_timestamp, timestamp_to_datetime};
use crate::{Error, Result};
use rusqlite::{Connection, params};

pub struct NodeRepository;

impl NodeRepository {
    /// Create a new outline node
    pub fn create(conn: &Connection, node: &OutlineNode) -> Result<()> {
        conn.execute(
            "INSERT INTO outline_nodes (id, note_id, parent_node_id, content, position, is_task, 
             task_completed, task_priority, task_due_date, block_type, created_at, modified_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                node.id,
                node.note_id,
                node.parent_node_id,
                node.content,
                node.position,
                node.is_task,
                node.task_completed,
                node.task_priority.as_ref().map(|p| p.to_string()),
                node.task_due_date.as_ref().map(datetime_to_timestamp),
                match &node.block_type {
                    BlockType::Normal => "normal",
                    BlockType::Quote => "quote",
                    BlockType::Code => "code",
                },
                datetime_to_timestamp(&node.created_at),
                datetime_to_timestamp(&node.modified_at),
            ],
        )?;
        Ok(())
    }

    /// Get a node by ID
    pub fn get_by_id(conn: &Connection, id: &str) -> Result<OutlineNode> {
        let mut stmt = conn.prepare(
            "SELECT id, note_id, parent_node_id, content, position, is_task, task_completed, 
             task_priority, task_due_date, block_type, created_at, modified_at FROM outline_nodes WHERE id = ?1"
        )?;
        
        let node = stmt.query_row(params![id], |row| {
            Ok(OutlineNode {
                id: row.get(0)?,
                note_id: row.get(1)?,
                parent_node_id: row.get(2)?,
                content: row.get(3)?,
                position: row.get(4)?,
                is_task: row.get(5)?,
                task_completed: row.get(6)?,
                task_priority: row.get::<_, Option<String>>(7)?
                    .and_then(|s| TaskPriority::from_str(&s)),
                task_due_date: row.get::<_, Option<i64>>(8)?
                    .map(timestamp_to_datetime),
                block_type: match row.get::<_, String>(9)?.as_str() {
                    "quote" => BlockType::Quote,
                    "code" => BlockType::Code,
                    _ => BlockType::Normal,
                },
                created_at: timestamp_to_datetime(row.get(10)?),
                modified_at: timestamp_to_datetime(row.get(11)?),
            })
        })?;
        
        Ok(node)
    }

    /// Get all nodes for a note
    pub fn get_by_note_id(conn: &Connection, note_id: &str) -> Result<Vec<OutlineNode>> {
        let mut stmt = conn.prepare(
            "SELECT id, note_id, parent_node_id, content, position, is_task, task_completed, 
             task_priority, task_due_date, block_type, created_at, modified_at FROM outline_nodes 
             WHERE note_id = ?1 ORDER BY position"
        )?;
        
        let nodes = stmt.query_map(params![note_id], |row| {
            Ok(OutlineNode {
                id: row.get(0)?,
                note_id: row.get(1)?,
                parent_node_id: row.get(2)?,
                content: row.get(3)?,
                position: row.get(4)?,
                is_task: row.get(5)?,
                task_completed: row.get(6)?,
                task_priority: row.get::<_, Option<String>>(7)?
                    .and_then(|s| TaskPriority::from_str(&s)),
                task_due_date: row.get::<_, Option<i64>>(8)?
                    .map(timestamp_to_datetime),
                block_type: match row.get::<_, String>(9)?.as_str() {
                    "quote" => BlockType::Quote,
                    "code" => BlockType::Code,
                    _ => BlockType::Normal,
                },
                created_at: timestamp_to_datetime(row.get(10)?),
                modified_at: timestamp_to_datetime(row.get(11)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(nodes)
    }

    /// Get child nodes of a parent
    pub fn get_children(conn: &Connection, parent_id: &str) -> Result<Vec<OutlineNode>> {
        let mut stmt = conn.prepare(
            "SELECT id, note_id, parent_node_id, content, position, is_task, task_completed, 
             task_priority, task_due_date, block_type, created_at, modified_at FROM outline_nodes 
             WHERE parent_node_id = ?1 ORDER BY position"
        )?;
        
        let nodes = stmt.query_map(params![parent_id], |row| {
            Ok(OutlineNode {
                id: row.get(0)?,
                note_id: row.get(1)?,
                parent_node_id: row.get(2)?,
                content: row.get(3)?,
                position: row.get(4)?,
                is_task: row.get(5)?,
                task_completed: row.get(6)?,
                task_priority: row.get::<_, Option<String>>(7)?
                    .and_then(|s| TaskPriority::from_str(&s)),
                task_due_date: row.get::<_, Option<i64>>(8)?
                    .map(timestamp_to_datetime),
                block_type: match row.get::<_, String>(9)?.as_str() {
                    "quote" => BlockType::Quote,
                    "code" => BlockType::Code,
                    _ => BlockType::Normal,
                },
                created_at: timestamp_to_datetime(row.get(10)?),
                modified_at: timestamp_to_datetime(row.get(11)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(nodes)
    }

    /// Get root nodes for a note (nodes with no parent)
    pub fn get_root_nodes(conn: &Connection, note_id: &str) -> Result<Vec<OutlineNode>> {
        let mut stmt = conn.prepare(
            "SELECT id, note_id, parent_node_id, content, position, is_task, task_completed, 
             task_priority, task_due_date, block_type, created_at, modified_at FROM outline_nodes 
             WHERE note_id = ?1 AND parent_node_id IS NULL ORDER BY position"
        )?;
        
        let nodes = stmt.query_map(params![note_id], |row| {
            Ok(OutlineNode {
                id: row.get(0)?,
                note_id: row.get(1)?,
                parent_node_id: row.get(2)?,
                content: row.get(3)?,
                position: row.get(4)?,
                is_task: row.get(5)?,
                task_completed: row.get(6)?,
                task_priority: row.get::<_, Option<String>>(7)?
                    .and_then(|s| TaskPriority::from_str(&s)),
                task_due_date: row.get::<_, Option<i64>>(8)?
                    .map(timestamp_to_datetime),
                block_type: match row.get::<_, String>(9)?.as_str() {
                    "quote" => BlockType::Quote,
                    "code" => BlockType::Code,
                    _ => BlockType::Normal,
                },
                created_at: timestamp_to_datetime(row.get(10)?),
                modified_at: timestamp_to_datetime(row.get(11)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(nodes)
    }

    /// Update a node
    pub fn update(conn: &Connection, node: &OutlineNode) -> Result<()> {
        let rows_affected = conn.execute(
            "UPDATE outline_nodes SET content = ?1, position = ?2, is_task = ?3, 
             task_completed = ?4, task_priority = ?5, task_due_date = ?6, block_type = ?7, modified_at = ?8 
             WHERE id = ?9",
            params![
                node.content,
                node.position,
                node.is_task,
                node.task_completed,
                node.task_priority.as_ref().map(|p| p.to_string()),
                node.task_due_date.as_ref().map(datetime_to_timestamp),
                match &node.block_type {
                    BlockType::Normal => "normal",
                    BlockType::Quote => "quote",
                    BlockType::Code => "code",
                },
                datetime_to_timestamp(&node.modified_at),
                node.id,
            ],
        )?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Node not found: {}", node.id)));
        }
        
        Ok(())
    }

    /// Delete a node
    pub fn delete(conn: &Connection, id: &str) -> Result<()> {
        let rows_affected = conn.execute("DELETE FROM outline_nodes WHERE id = ?1", params![id])?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Node not found: {}", id)));
        }
        
        Ok(())
    }

    /// Search nodes by content using FTS5
    pub fn search(conn: &Connection, query: &str) -> Result<Vec<OutlineNode>> {
        let mut stmt = conn.prepare(
            "SELECT n.id, n.note_id, n.parent_node_id, n.content, n.position, n.is_task, 
             n.task_completed, n.task_priority, n.task_due_date, n.block_type, n.created_at, n.modified_at 
             FROM outline_nodes n 
             INNER JOIN nodes_fts fts ON fts.node_id = n.id 
             WHERE nodes_fts MATCH ?1"
        )?;
        
        let nodes = stmt.query_map(params![query], |row| {
            Ok(OutlineNode {
                id: row.get(0)?,
                note_id: row.get(1)?,
                parent_node_id: row.get(2)?,
                content: row.get(3)?,
                position: row.get(4)?,
                is_task: row.get(5)?,
                task_completed: row.get(6)?,
                task_priority: row.get::<_, Option<String>>(7)?
                    .and_then(|s| TaskPriority::from_str(&s)),
                task_due_date: row.get::<_, Option<i64>>(8)?
                    .map(timestamp_to_datetime),
                block_type: match row.get::<_, String>(9)?.as_str() {
                    "quote" => BlockType::Quote,
                    "code" => BlockType::Code,
                    _ => BlockType::Normal,
                },
                created_at: timestamp_to_datetime(row.get(10)?),
                modified_at: timestamp_to_datetime(row.get(11)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(nodes)
    }

    /// Get all tasks (optionally filter by completion status)
    pub fn get_tasks(conn: &Connection, completed: Option<bool>) -> Result<Vec<OutlineNode>> {
        let query = match completed {
            Some(true) => "SELECT id, note_id, parent_node_id, content, position, is_task, 
                          task_completed, task_priority, task_due_date, block_type, created_at, modified_at 
                          FROM outline_nodes WHERE is_task = 1 AND task_completed = 1 ORDER BY modified_at DESC",
            Some(false) => "SELECT id, note_id, parent_node_id, content, position, is_task, 
                           task_completed, task_priority, task_due_date, block_type, created_at, modified_at 
                           FROM outline_nodes WHERE is_task = 1 AND task_completed = 0 ORDER BY task_due_date",
            None => "SELECT id, note_id, parent_node_id, content, position, is_task, 
                    task_completed, task_priority, task_due_date, block_type, created_at, modified_at 
                    FROM outline_nodes WHERE is_task = 1 ORDER BY task_due_date",
        };
        
        let mut stmt = conn.prepare(query)?;
        
        let nodes = stmt.query_map([], |row| {
            Ok(OutlineNode {
                id: row.get(0)?,
                note_id: row.get(1)?,
                parent_node_id: row.get(2)?,
                content: row.get(3)?,
                position: row.get(4)?,
                is_task: row.get(5)?,
                task_completed: row.get(6)?,
                task_priority: row.get::<_, Option<String>>(7)?
                    .and_then(|s| TaskPriority::from_str(&s)),
                task_due_date: row.get::<_, Option<i64>>(8)?
                    .map(timestamp_to_datetime),
                block_type: match row.get::<_, String>(9)?.as_str() {
                    "quote" => BlockType::Quote,
                    "code" => BlockType::Code,
                    _ => BlockType::Normal,
                },
                created_at: timestamp_to_datetime(row.get(10)?),
                modified_at: timestamp_to_datetime(row.get(11)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(nodes)
    }

    /// Update a node's parent and position in one operation
    pub fn update_parent_and_position(
        conn: &Connection,
        id: &str,
        new_parent_node_id: Option<&str>,
        new_position: i32,
    ) -> Result<()> {
        let rows_affected = conn.execute(
            "UPDATE outline_nodes SET parent_node_id = ?1, position = ?2, modified_at = ?3 WHERE id = ?4",
            params![
                new_parent_node_id,
                new_position,
                datetime_to_timestamp(&chrono::Utc::now()),
                id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Node not found: {}", id)));
        }

        Ok(())
    }

    /// Swap the `position` values for two sibling nodes
    pub fn swap_positions(conn: &Connection, id_a: &str, id_b: &str) -> Result<()> {
        let node_a = Self::get_by_id(conn, id_a)?;
        let node_b = Self::get_by_id(conn, id_b)?;

        // Only allow swap if siblings (same parent and note)
        if node_a.note_id != node_b.note_id || node_a.parent_node_id != node_b.parent_node_id {
            return Err(Error::InvalidInput("Nodes are not siblings; cannot swap positions".to_string()));
        }

        // Use a transaction to keep positions consistent
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE outline_nodes SET position = ?1, modified_at = ?2 WHERE id = ?3",
            params![node_b.position, datetime_to_timestamp(&chrono::Utc::now()), id_a],
        )?;
        tx.execute(
            "UPDATE outline_nodes SET position = ?1, modified_at = ?2 WHERE id = ?3",
            params![node_a.position, datetime_to_timestamp(&chrono::Utc::now()), id_b],
        )?;
        tx.commit()?;

        Ok(())
    }

    /// Get the next position index for a parent's children (append to end)
    pub fn get_next_child_position(conn: &Connection, parent_node_id: Option<&str>, note_id: &str) -> Result<i32> {
        let query = match parent_node_id {
            Some(_) => "SELECT COALESCE(MAX(position), -1) + 1 FROM outline_nodes WHERE parent_node_id = ?1",
            None => "SELECT COALESCE(MAX(position), -1) + 1 FROM outline_nodes WHERE note_id = ?1 AND parent_node_id IS NULL",
        };

        let mut stmt = conn.prepare(query)?;
        let next_pos: i32 = match parent_node_id {
            Some(pid) => stmt.query_row(params![pid], |row| row.get(0))?,
            None => stmt.query_row(params![note_id], |row| row.get(0))?,
        };
        Ok(next_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Note;
    use crate::storage::{Database, NoteRepository};
    use tempfile::tempdir;

    fn setup_test_db() -> (tempfile::TempDir, Connection, Note) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path);
        let conn = db.create().unwrap();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        (dir, conn, note)
    }

    #[test]
    fn test_create_node() {
        let (_dir, conn, note) = setup_test_db();
        let node = OutlineNode::new(note.id.clone(), None, "Test content".to_string(), 0);
        
        NodeRepository::create(&conn, &node).unwrap();
        
        let retrieved = NodeRepository::get_by_id(&conn, &node.id).unwrap();
        assert_eq!(retrieved.content, "Test content");
    }

    #[test]
    fn test_get_by_note_id() {
        let (_dir, conn, note) = setup_test_db();
        
        let node1 = OutlineNode::new(note.id.clone(), None, "Node 1".to_string(), 0);
        let node2 = OutlineNode::new(note.id.clone(), None, "Node 2".to_string(), 1);
        
        NodeRepository::create(&conn, &node1).unwrap();
        NodeRepository::create(&conn, &node2).unwrap();
        
        let nodes = NodeRepository::get_by_note_id(&conn, &note.id).unwrap();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_get_children() {
        let (_dir, conn, note) = setup_test_db();
        
        let parent = OutlineNode::new(note.id.clone(), None, "Parent".to_string(), 0);
        NodeRepository::create(&conn, &parent).unwrap();
        
        let child1 = OutlineNode::new(note.id.clone(), Some(parent.id.clone()), "Child 1".to_string(), 0);
        let child2 = OutlineNode::new(note.id.clone(), Some(parent.id.clone()), "Child 2".to_string(), 1);
        
        NodeRepository::create(&conn, &child1).unwrap();
        NodeRepository::create(&conn, &child2).unwrap();
        
        let children = NodeRepository::get_children(&conn, &parent.id).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_update_node() {
        let (_dir, conn, note) = setup_test_db();
        let mut node = OutlineNode::new(note.id.clone(), None, "Original".to_string(), 0);
        
        NodeRepository::create(&conn, &node).unwrap();
        
        node.content = "Updated".to_string();
        node.touch();
        NodeRepository::update(&conn, &node).unwrap();
        
        let retrieved = NodeRepository::get_by_id(&conn, &node.id).unwrap();
        assert_eq!(retrieved.content, "Updated");
    }

    #[test]
    fn test_delete_node() {
        let (_dir, conn, note) = setup_test_db();
        let node = OutlineNode::new(note.id.clone(), None, "To Delete".to_string(), 0);
        
        NodeRepository::create(&conn, &node).unwrap();
        NodeRepository::delete(&conn, &node.id).unwrap();
        
        let result = NodeRepository::get_by_id(&conn, &node.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_task_operations() {
        let (_dir, conn, note) = setup_test_db();
        
        let task = OutlineNode::new_task(
            note.id.clone(),
            None,
            "Task content".to_string(),
            0,
            Some(TaskPriority::High),
            None,
        );
        
        NodeRepository::create(&conn, &task).unwrap();
        
        let tasks = NodeRepository::get_tasks(&conn, Some(false)).unwrap();
        assert_eq!(tasks.len(), 1);
        
        let tasks_completed = NodeRepository::get_tasks(&conn, Some(true)).unwrap();
        assert_eq!(tasks_completed.len(), 0);
    }
}

