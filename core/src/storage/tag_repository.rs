use crate::models::{Tag, datetime_to_timestamp, timestamp_to_datetime};
use crate::{Error, Result};
use rusqlite::{Connection, params};

pub struct TagRepository;

impl TagRepository {
    /// Create a new tag
    pub fn create(conn: &Connection, tag: &Tag) -> Result<i64> {
        conn.execute(
            "INSERT INTO tags (name, color, created_at) VALUES (?1, ?2, ?3)",
            params![
                tag.name,
                tag.color,
                datetime_to_timestamp(&tag.created_at),
            ],
        )?;
        
        Ok(conn.last_insert_rowid())
    }

    /// Get a tag by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Tag> {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, created_at FROM tags WHERE id = ?1"
        )?;
        
        let tag = stmt.query_row(params![id], |row| {
            Ok(Tag {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: timestamp_to_datetime(row.get(3)?),
            })
        })?;
        
        Ok(tag)
    }

    /// Get a tag by name
    pub fn get_by_name(conn: &Connection, name: &str) -> Result<Tag> {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, created_at FROM tags WHERE name = ?1"
        )?;
        
        let tag = stmt.query_row(params![name], |row| {
            Ok(Tag {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: timestamp_to_datetime(row.get(3)?),
            })
        })?;
        
        Ok(tag)
    }

    /// Get or create a tag by name
    pub fn get_or_create(conn: &Connection, name: &str, color: Option<String>) -> Result<Tag> {
        match Self::get_by_name(conn, name) {
            Ok(tag) => Ok(tag),
            Err(Error::Database(rusqlite::Error::QueryReturnedNoRows)) => {
                let mut new_tag = Tag::new(name.to_string(), color);
                let id = Self::create(conn, &new_tag)?;
                new_tag.id = Some(id);
                Ok(new_tag)
            }
            Err(e) => Err(e),
        }
    }

    /// Get all tags
    pub fn get_all(conn: &Connection) -> Result<Vec<Tag>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, created_at FROM tags ORDER BY name"
        )?;
        
        let tags = stmt.query_map([], |row| {
            Ok(Tag {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: timestamp_to_datetime(row.get(3)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(tags)
    }

    /// Delete a tag
    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        let rows_affected = conn.execute("DELETE FROM tags WHERE id = ?1", params![id])?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Tag not found: {}", id)));
        }
        
        Ok(())
    }

    /// Add a tag to a node
    pub fn add_to_node(conn: &Connection, node_id: &str, tag_id: i64) -> Result<()> {
        let now = chrono::Utc::now();
        conn.execute(
            "INSERT OR IGNORE INTO node_tags (node_id, tag_id, created_at) VALUES (?1, ?2, ?3)",
            params![node_id, tag_id, datetime_to_timestamp(&now)],
        )?;
        Ok(())
    }

    /// Remove a tag from a node
    pub fn remove_from_node(conn: &Connection, node_id: &str, tag_id: i64) -> Result<()> {
        conn.execute(
            "DELETE FROM node_tags WHERE node_id = ?1 AND tag_id = ?2",
            params![node_id, tag_id],
        )?;
        Ok(())
    }

    /// Get all tags for a node
    pub fn get_for_node(conn: &Connection, node_id: &str) -> Result<Vec<Tag>> {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.color, t.created_at 
             FROM tags t 
             INNER JOIN node_tags nt ON nt.tag_id = t.id 
             WHERE nt.node_id = ?1 
             ORDER BY t.name"
        )?;
        
        let tags = stmt.query_map(params![node_id], |row| {
            Ok(Tag {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: timestamp_to_datetime(row.get(3)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(tags)
    }

    /// Get tag usage count
    pub fn get_usage_counts(conn: &Connection) -> Result<Vec<(Tag, i64)>> {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.color, t.created_at, COUNT(nt.node_id) as usage_count 
             FROM tags t 
             LEFT JOIN node_tags nt ON nt.tag_id = t.id 
             GROUP BY t.id 
             ORDER BY usage_count DESC, t.name"
        )?;
        
        let results = stmt.query_map([], |row| {
            let tag = Tag {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: timestamp_to_datetime(row.get(3)?),
            };
            let count: i64 = row.get(4)?;
            Ok((tag, count))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(results)
    }

    /// Get distinct note IDs that contain at least one node with the given tag name
    pub fn get_note_ids_for_tag_name(conn: &Connection, tag_name: &str) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT n.note_id \
             FROM node_tags nt \
             INNER JOIN tags t ON t.id = nt.tag_id \
             INNER JOIN outline_nodes n ON n.id = nt.node_id \
             WHERE t.name = ?1"
        )?;

        let note_ids = stmt.query_map(params![tag_name], |row| {
            let id: String = row.get(0)?;
            Ok(id)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(note_ids)
    }

    /// Remove all tag associations from a node
    pub fn remove_all_from_node(conn: &Connection, node_id: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM node_tags WHERE node_id = ?1",
            params![node_id],
        )?;
        Ok(())
    }

    /// Set tags for a node to exactly the provided tag names (creates tags as needed)
    pub fn set_tags_for_node(conn: &Connection, node_id: &str, tag_names: &[String]) -> Result<()> {
        // Start by clearing existing associations
        Self::remove_all_from_node(conn, node_id)?;
        // Add each tag by name, creating if necessary
        for name in tag_names {
            let tag = Self::get_or_create(conn, name, None)?;
            if let Some(tag_id) = tag.id { Self::add_to_node(conn, node_id, tag_id)?; }
        }
        Ok(())
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
    fn test_create_tag() {
        let (_dir, conn) = setup_test_db();
        let tag = Tag::new("work".to_string(), Some("#FF5733".to_string()));
        
        let id = TagRepository::create(&conn, &tag).unwrap();
        assert!(id > 0);
        
        let retrieved = TagRepository::get_by_id(&conn, id).unwrap();
        assert_eq!(retrieved.name, "work");
    }

    #[test]
    fn test_get_or_create() {
        let (_dir, conn) = setup_test_db();
        
        let tag1 = TagRepository::get_or_create(&conn, "project", None).unwrap();
        let tag2 = TagRepository::get_or_create(&conn, "project", None).unwrap();
        
        assert_eq!(tag1.id, tag2.id);
    }

    #[test]
    fn test_add_tag_to_node() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let node = OutlineNode::new(note.id.clone(), None, "Content".to_string(), 0);
        NodeRepository::create(&conn, &node).unwrap();
        
        let tag = Tag::new("test-tag".to_string(), None);
        let tag_id = TagRepository::create(&conn, &tag).unwrap();
        
        TagRepository::add_to_node(&conn, &node.id, tag_id).unwrap();
        
        let tags = TagRepository::get_for_node(&conn, &node.id).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "test-tag");
    }

    #[test]
    fn test_usage_counts() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let node1 = OutlineNode::new(note.id.clone(), None, "Node 1".to_string(), 0);
        let node2 = OutlineNode::new(note.id.clone(), None, "Node 2".to_string(), 1);
        NodeRepository::create(&conn, &node1).unwrap();
        NodeRepository::create(&conn, &node2).unwrap();
        
        let tag = Tag::new("popular".to_string(), None);
        let tag_id = TagRepository::create(&conn, &tag).unwrap();
        
        TagRepository::add_to_node(&conn, &node1.id, tag_id).unwrap();
        TagRepository::add_to_node(&conn, &node2.id, tag_id).unwrap();
        
        let counts = TagRepository::get_usage_counts(&conn).unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].1, 2); // Used twice
    }
}

