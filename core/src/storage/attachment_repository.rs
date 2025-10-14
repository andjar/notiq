use crate::models::{Attachment, datetime_to_timestamp, timestamp_to_datetime};
use crate::{Error, Result};
use rusqlite::{Connection, params};

pub struct AttachmentRepository;

impl AttachmentRepository {
    /// Create a new attachment
    pub fn create(conn: &Connection, attachment: &Attachment) -> Result<()> {
        conn.execute(
            "INSERT INTO attachments (id, note_id, node_id, filename, filepath, mime_type, size_bytes, hash, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                attachment.id,
                attachment.note_id,
                attachment.node_id,
                attachment.filename,
                attachment.filepath,
                attachment.mime_type,
                attachment.size_bytes,
                attachment.hash,
                datetime_to_timestamp(&attachment.created_at),
            ],
        )?;
        
        Ok(())
    }

    /// Get an attachment by ID
    pub fn get_by_id(conn: &Connection, id: &str) -> Result<Attachment> {
        let mut stmt = conn.prepare(
            "SELECT id, note_id, node_id, filename, filepath, mime_type, size_bytes, hash, created_at 
             FROM attachments WHERE id = ?1"
        )?;
        
        let attachment = stmt.query_row(params![id], |row| {
            Ok(Attachment {
                id: row.get(0)?,
                note_id: row.get(1)?,
                node_id: row.get(2)?,
                filename: row.get(3)?,
                filepath: row.get(4)?,
                mime_type: row.get(5)?,
                size_bytes: row.get(6)?,
                hash: row.get(7)?,
                created_at: timestamp_to_datetime(row.get(8)?),
            })
        })?;
        
        Ok(attachment)
    }

    /// Get all attachments for a note
    pub fn get_by_note_id(conn: &Connection, note_id: &str) -> Result<Vec<Attachment>> {
        let mut stmt = conn.prepare(
            "SELECT id, note_id, node_id, filename, filepath, mime_type, size_bytes, hash, created_at 
             FROM attachments WHERE note_id = ?1 ORDER BY created_at DESC"
        )?;
        
        let attachments = stmt.query_map(params![note_id], |row| {
            Ok(Attachment {
                id: row.get(0)?,
                note_id: row.get(1)?,
                node_id: row.get(2)?,
                filename: row.get(3)?,
                filepath: row.get(4)?,
                mime_type: row.get(5)?,
                size_bytes: row.get(6)?,
                hash: row.get(7)?,
                created_at: timestamp_to_datetime(row.get(8)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(attachments)
    }

    /// Get an attachment by hash (for deduplication)
    pub fn get_by_hash(conn: &Connection, hash: &str) -> Result<Option<Attachment>> {
        let mut stmt = conn.prepare(
            "SELECT id, note_id, node_id, filename, filepath, mime_type, size_bytes, hash, created_at 
             FROM attachments WHERE hash = ?1 LIMIT 1"
        )?;
        
        let result = stmt.query_row(params![hash], |row| {
            Ok(Attachment {
                id: row.get(0)?,
                note_id: row.get(1)?,
                node_id: row.get(2)?,
                filename: row.get(3)?,
                filepath: row.get(4)?,
                mime_type: row.get(5)?,
                size_bytes: row.get(6)?,
                hash: row.get(7)?,
                created_at: timestamp_to_datetime(row.get(8)?),
            })
        });
        
        match result {
            Ok(attachment) => Ok(Some(attachment)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(e)),
        }
    }

    /// Delete an attachment
    pub fn delete(conn: &Connection, id: &str) -> Result<()> {
        let rows_affected = conn.execute("DELETE FROM attachments WHERE id = ?1", params![id])?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Attachment not found: {}", id)));
        }
        
        Ok(())
    }

    /// Get total size of all attachments
    pub fn get_total_size(conn: &Connection) -> Result<i64> {
        let size: Option<i64> = conn.query_row(
            "SELECT SUM(size_bytes) FROM attachments",
            [],
            |row| row.get(0),
        )?;
        
        Ok(size.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Note, OutlineNode};
    use crate::storage::{Database, NodeRepository, NoteRepository};
    use tempfile::tempdir;

    fn setup_test_db() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path);
        let conn = db.create().unwrap();
        (dir, conn)
    }

    #[test]
    fn test_create_attachment() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        let node = OutlineNode::new(note.id.clone(), None, "".to_string(), 0);
        NodeRepository::create(&conn, &node).unwrap();
        
        let attachment = Attachment::new(
            note.id.clone(),
            node.id.clone(),
            "document.pdf".to_string(),
            "/path/to/document.pdf".to_string(),
            Some("application/pdf".to_string()),
            1024,
            "abc123".to_string(),
        );
        
        AttachmentRepository::create(&conn, &attachment).unwrap();
        
        let retrieved = AttachmentRepository::get_by_id(&conn, &attachment.id).unwrap();
        assert_eq!(retrieved.filename, "document.pdf");
    }

    #[test]
    fn test_get_by_note_id() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        let node = OutlineNode::new(note.id.clone(), None, "".to_string(), 0);
        NodeRepository::create(&conn, &node).unwrap();
        
        let attachment1 = Attachment::new(
            note.id.clone(),
            node.id.clone(),
            "file1.txt".to_string(),
            "/path/file1.txt".to_string(),
            None,
            100,
            "hash1".to_string(),
        );
        
        let attachment2 = Attachment::new(
            note.id.clone(),
            node.id.clone(),
            "file2.txt".to_string(),
            "/path/file2.txt".to_string(),
            None,
            200,
            "hash2".to_string(),
        );
        
        AttachmentRepository::create(&conn, &attachment1).unwrap();
        AttachmentRepository::create(&conn, &attachment2).unwrap();
        
        let attachments = AttachmentRepository::get_by_note_id(&conn, &note.id).unwrap();
        assert_eq!(attachments.len(), 2);
    }

    #[test]
    fn test_get_by_hash() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        let node = OutlineNode::new(note.id.clone(), None, "".to_string(), 0);
        NodeRepository::create(&conn, &node).unwrap();
        
        let attachment = Attachment::new(
            note.id.clone(),
            node.id.clone(),
            "file.txt".to_string(),
            "/path/file.txt".to_string(),
            None,
            100,
            "unique-hash".to_string(),
        );
        
        AttachmentRepository::create(&conn, &attachment).unwrap();
        
        let found = AttachmentRepository::get_by_hash(&conn, "unique-hash").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().filename, "file.txt");
        
        let not_found = AttachmentRepository::get_by_hash(&conn, "nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_total_size() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        let node = OutlineNode::new(note.id.clone(), None, "".to_string(), 0);
        NodeRepository::create(&conn, &node).unwrap();
        
        let attachment1 = Attachment::new(
            note.id.clone(),
            node.id.clone(),
            "file1.txt".to_string(),
            "/path/file1.txt".to_string(),
            None,
            1000,
            "hash1".to_string(),
        );
        
        let attachment2 = Attachment::new(
            note.id.clone(),
            node.id.clone(),
            "file2.txt".to_string(),
            "/path/file2.txt".to_string(),
            None,
            2000,
            "hash2".to_string(),
        );
        
        AttachmentRepository::create(&conn, &attachment1).unwrap();
        AttachmentRepository::create(&conn, &attachment2).unwrap();
        
        let total_size = AttachmentRepository::get_total_size(&conn).unwrap();
        assert_eq!(total_size, 3000);
    }
}

