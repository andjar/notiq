use crate::models::{Note, datetime_to_timestamp, timestamp_to_datetime};
use crate::{Error, Result};
use rusqlite::{Connection, params};

pub struct NoteRepository;

impl NoteRepository {
    /// Create a new note
    pub fn create(conn: &Connection, note: &Note) -> Result<()> {
        conn.execute(
            "INSERT INTO notes (id, title, created_at, modified_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                note.id,
                note.title,
                datetime_to_timestamp(&note.created_at),
                datetime_to_timestamp(&note.modified_at),
            ],
        )?;
        Ok(())
    }

    /// Get a note by ID
    pub fn get_by_id(conn: &Connection, id: &str) -> Result<Note> {
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, modified_at FROM notes WHERE id = ?1"
        )?;
        
        let note = stmt.query_row(params![id], |row| {
            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: timestamp_to_datetime(row.get(2)?),
                modified_at: timestamp_to_datetime(row.get(3)?),
            })
        })?;
        
        Ok(note)
    }

    /// Get all notes
    pub fn get_all(conn: &Connection) -> Result<Vec<Note>> {
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, modified_at FROM notes ORDER BY modified_at DESC"
        )?;
        
        let notes = stmt.query_map([], |row| {
            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: timestamp_to_datetime(row.get(2)?),
                modified_at: timestamp_to_datetime(row.get(3)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(notes)
    }

    /// Update a note
    pub fn update(conn: &Connection, note: &Note) -> Result<()> {
        let rows_affected = conn.execute(
            "UPDATE notes SET title = ?1, modified_at = ?2 WHERE id = ?3",
            params![
                note.title,
                datetime_to_timestamp(&note.modified_at),
                note.id,
            ],
        )?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Note not found: {}", note.id)));
        }
        
        Ok(())
    }

    /// Delete a note
    pub fn delete(conn: &Connection, id: &str) -> Result<()> {
        let rows_affected = conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Note not found: {}", id)));
        }
        
        Ok(())
    }

    /// Search notes by title
    pub fn search_by_title(conn: &Connection, query: &str) -> Result<Vec<Note>> {
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, modified_at FROM notes WHERE title LIKE ?1 ORDER BY modified_at DESC"
        )?;
        
        let search_pattern = format!("%{}%", query);
        let notes = stmt.query_map(params![search_pattern], |row| {
            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: timestamp_to_datetime(row.get(2)?),
                modified_at: timestamp_to_datetime(row.get(3)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(notes)
    }

    /// Count total notes
    pub fn count(conn: &Connection) -> Result<i64> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Get a note by exact title match (case-sensitive)
    pub fn get_by_title_exact(conn: &Connection, title: &str) -> Result<Note> {
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, modified_at FROM notes WHERE title = ?1"
        )?;

        let note = stmt.query_row(params![title], |row| {
            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: timestamp_to_datetime(row.get(2)?),
                modified_at: timestamp_to_datetime(row.get(3)?),
            })
        })?;

        Ok(note)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Note;
    use crate::storage::Database;
    use tempfile::tempdir;

    fn setup_test_db() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(&db_path);
        let conn = db.create().unwrap();
        (dir, conn)
    }

    #[test]
    fn test_create_note() {
        let (_dir, conn) = setup_test_db();
        let note = Note::new("Test Note".to_string());
        
        NoteRepository::create(&conn, &note).unwrap();
        
        let retrieved = NoteRepository::get_by_id(&conn, &note.id).unwrap();
        assert_eq!(retrieved.title, "Test Note");
    }

    #[test]
    fn test_get_all_notes() {
        let (_dir, conn) = setup_test_db();
        
        let note1 = Note::new("Note 1".to_string());
        let note2 = Note::new("Note 2".to_string());
        
        NoteRepository::create(&conn, &note1).unwrap();
        NoteRepository::create(&conn, &note2).unwrap();
        
        let notes = NoteRepository::get_all(&conn).unwrap();
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn test_update_note() {
        let (_dir, conn) = setup_test_db();
        let mut note = Note::new("Original Title".to_string());
        
        NoteRepository::create(&conn, &note).unwrap();
        
        note.title = "Updated Title".to_string();
        note.touch();
        NoteRepository::update(&conn, &note).unwrap();
        
        let retrieved = NoteRepository::get_by_id(&conn, &note.id).unwrap();
        assert_eq!(retrieved.title, "Updated Title");
    }

    #[test]
    fn test_delete_note() {
        let (_dir, conn) = setup_test_db();
        let note = Note::new("To Delete".to_string());
        
        NoteRepository::create(&conn, &note).unwrap();
        NoteRepository::delete(&conn, &note.id).unwrap();
        
        let result = NoteRepository::get_by_id(&conn, &note.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_by_title() {
        let (_dir, conn) = setup_test_db();
        
        let note1 = Note::new("Project Planning".to_string());
        let note2 = Note::new("Meeting Notes".to_string());
        let note3 = Note::new("Project Ideas".to_string());
        
        NoteRepository::create(&conn, &note1).unwrap();
        NoteRepository::create(&conn, &note2).unwrap();
        NoteRepository::create(&conn, &note3).unwrap();
        
        let results = NoteRepository::search_by_title(&conn, "Project").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_count_notes() {
        let (_dir, conn) = setup_test_db();
        
        assert_eq!(NoteRepository::count(&conn).unwrap(), 0);
        
        let note = Note::new("Test".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        assert_eq!(NoteRepository::count(&conn).unwrap(), 1);
    }
}

