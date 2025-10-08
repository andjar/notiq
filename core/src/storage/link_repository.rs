use crate::models::{Link, LinkType, datetime_to_timestamp, timestamp_to_datetime};
use crate::{Error, Result};
use rusqlite::{Connection, params};

pub struct LinkRepository;

impl LinkRepository {
    /// Create a new link
    pub fn create(conn: &Connection, link: &Link) -> Result<i64> {
        conn.execute(
            "INSERT INTO links (source_note_id, source_node_id, target_note_id, link_text, link_type, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                link.source_note_id,
                link.source_node_id,
                link.target_note_id,
                link.link_text,
                link.link_type.to_string(),
                datetime_to_timestamp(&link.created_at),
            ],
        )?;
        
        Ok(conn.last_insert_rowid())
    }

    /// Get a link by ID
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Link> {
        let mut stmt = conn.prepare(
            "SELECT id, source_note_id, source_node_id, target_note_id, link_text, link_type, created_at 
             FROM links WHERE id = ?1"
        )?;
        
        let link = stmt.query_row(params![id], |row| {
            Ok(Link {
                id: Some(row.get(0)?),
                source_note_id: row.get(1)?,
                source_node_id: row.get(2)?,
                target_note_id: row.get(3)?,
                link_text: row.get(4)?,
                link_type: LinkType::from_str(&row.get::<_, String>(5)?)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                created_at: timestamp_to_datetime(row.get(6)?),
            })
        })?;
        
        Ok(link)
    }

    /// Get all links from a source note
    pub fn get_by_source_note(conn: &Connection, source_note_id: &str) -> Result<Vec<Link>> {
        let mut stmt = conn.prepare(
            "SELECT id, source_note_id, source_node_id, target_note_id, link_text, link_type, created_at 
             FROM links WHERE source_note_id = ?1"
        )?;
        
        let links = stmt.query_map(params![source_note_id], |row| {
            Ok(Link {
                id: Some(row.get(0)?),
                source_note_id: row.get(1)?,
                source_node_id: row.get(2)?,
                target_note_id: row.get(3)?,
                link_text: row.get(4)?,
                link_type: LinkType::from_str(&row.get::<_, String>(5)?)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                created_at: timestamp_to_datetime(row.get(6)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(links)
    }

    /// Get all backlinks to a target note
    pub fn get_backlinks(conn: &Connection, target_note_id: &str) -> Result<Vec<Link>> {
        let mut stmt = conn.prepare(
            "SELECT id, source_note_id, source_node_id, target_note_id, link_text, link_type, created_at 
             FROM links WHERE target_note_id = ?1"
        )?;
        
        let links = stmt.query_map(params![target_note_id], |row| {
            Ok(Link {
                id: Some(row.get(0)?),
                source_note_id: row.get(1)?,
                source_node_id: row.get(2)?,
                target_note_id: row.get(3)?,
                link_text: row.get(4)?,
                link_type: LinkType::from_str(&row.get::<_, String>(5)?)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                created_at: timestamp_to_datetime(row.get(6)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(links)
    }

    /// Delete a link
    pub fn delete(conn: &Connection, id: i64) -> Result<()> {
        let rows_affected = conn.execute("DELETE FROM links WHERE id = ?1", params![id])?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Link not found: {}", id)));
        }
        
        Ok(())
    }

    /// Delete all links originating from a specific source node
    pub fn delete_by_source_node(conn: &Connection, source_node_id: &str) -> Result<usize> {
        let rows_affected = conn.execute(
            "DELETE FROM links WHERE source_node_id = ?1",
            params![source_node_id],
        )?;
        Ok(rows_affected)
    }

    /// Delete all links from a source note
    pub fn delete_by_source_note(conn: &Connection, source_note_id: &str) -> Result<usize> {
        let rows_affected = conn.execute(
            "DELETE FROM links WHERE source_note_id = ?1",
            params![source_note_id],
        )?;
        
        Ok(rows_affected)
    }

    /// Count backlinks to a note
    pub fn count_backlinks(conn: &Connection, target_note_id: &str) -> Result<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM links WHERE target_note_id = ?1",
            params![target_note_id],
            |row| row.get(0),
        )?;
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Note;
    use crate::storage::{Database, NoteRepository};
    use tempfile::tempdir;

    fn setup_test_db() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path);
        let conn = db.create().unwrap();
        (dir, conn)
    }

    #[test]
    fn test_create_link() {
        let (_dir, conn) = setup_test_db();
        
        let note1 = Note::new("Note 1".to_string());
        let note2 = Note::new("Note 2".to_string());
        NoteRepository::create(&conn, &note1).unwrap();
        NoteRepository::create(&conn, &note2).unwrap();
        
        let link = Link::new_wiki_link(
            note1.id.clone(),
            None,
            note2.id.clone(),
            Some("Link to Note 2".to_string()),
        );
        
        let id = LinkRepository::create(&conn, &link).unwrap();
        assert!(id > 0);
        
        let retrieved = LinkRepository::get_by_id(&conn, id).unwrap();
        assert_eq!(retrieved.source_note_id, note1.id);
        assert_eq!(retrieved.target_note_id, note2.id);
    }

    #[test]
    fn test_get_backlinks() {
        let (_dir, conn) = setup_test_db();
        
        let note1 = Note::new("Note 1".to_string());
        let note2 = Note::new("Note 2".to_string());
        let note3 = Note::new("Note 3".to_string());
        NoteRepository::create(&conn, &note1).unwrap();
        NoteRepository::create(&conn, &note2).unwrap();
        NoteRepository::create(&conn, &note3).unwrap();
        
        // Create links from note1 and note3 to note2
        let link1 = Link::new_wiki_link(note1.id.clone(), None, note2.id.clone(), None);
        let link2 = Link::new_wiki_link(note3.id.clone(), None, note2.id.clone(), None);
        
        LinkRepository::create(&conn, &link1).unwrap();
        LinkRepository::create(&conn, &link2).unwrap();
        
        let backlinks = LinkRepository::get_backlinks(&conn, &note2.id).unwrap();
        assert_eq!(backlinks.len(), 2);
        
        let count = LinkRepository::count_backlinks(&conn, &note2.id).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_delete_link() {
        let (_dir, conn) = setup_test_db();
        
        let note1 = Note::new("Note 1".to_string());
        let note2 = Note::new("Note 2".to_string());
        NoteRepository::create(&conn, &note1).unwrap();
        NoteRepository::create(&conn, &note2).unwrap();
        
        let link = Link::new_wiki_link(note1.id.clone(), None, note2.id.clone(), None);
        let id = LinkRepository::create(&conn, &link).unwrap();
        
        LinkRepository::delete(&conn, id).unwrap();
        
        let result = LinkRepository::get_by_id(&conn, id);
        assert!(result.is_err());
    }
}

