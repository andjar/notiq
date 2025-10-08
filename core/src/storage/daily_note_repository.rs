use crate::models::DailyNote;
use crate::{Error, Result};
use chrono::NaiveDate;
use rusqlite::{Connection, params};

pub struct DailyNoteRepository;

impl DailyNoteRepository {
    /// Create a daily note entry
    pub fn create(conn: &Connection, daily_note: &DailyNote) -> Result<()> {
        conn.execute(
            "INSERT INTO daily_notes (date, note_id) VALUES (?1, ?2)",
            params![daily_note.date_string(), daily_note.note_id],
        )?;
        
        Ok(())
    }

    /// Get a daily note by date
    pub fn get_by_date(conn: &Connection, date: NaiveDate) -> Result<DailyNote> {
        let date_str = date.format("%Y-%m-%d").to_string();
        
        let mut stmt = conn.prepare(
            "SELECT date, note_id FROM daily_notes WHERE date = ?1"
        )?;
        
        let daily_note = stmt.query_row(params![date_str], |row| {
            let date_string: String = row.get(0)?;
            let date = NaiveDate::parse_from_str(&date_string, "%Y-%m-%d")
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            
            Ok(DailyNote {
                date,
                note_id: row.get(1)?,
            })
        })?;
        
        Ok(daily_note)
    }

    /// Get or create a daily note for a specific date
    pub fn get_or_create(conn: &Connection, date: NaiveDate, note_id: String) -> Result<DailyNote> {
        match Self::get_by_date(conn, date) {
            Ok(daily_note) => Ok(daily_note),
            Err(Error::Database(rusqlite::Error::QueryReturnedNoRows)) => {
                let daily_note = DailyNote::new(date, note_id);
                Self::create(conn, &daily_note)?;
                Ok(daily_note)
            }
            Err(e) => Err(e),
        }
    }

    /// Get all daily notes
    pub fn get_all(conn: &Connection) -> Result<Vec<DailyNote>> {
        let mut stmt = conn.prepare(
            "SELECT date, note_id FROM daily_notes ORDER BY date DESC"
        )?;
        
        let daily_notes = stmt.query_map([], |row| {
            let date_string: String = row.get(0)?;
            let date = NaiveDate::parse_from_str(&date_string, "%Y-%m-%d")
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            
            Ok(DailyNote {
                date,
                note_id: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(daily_notes)
    }

    /// Delete a daily note entry
    pub fn delete(conn: &Connection, date: NaiveDate) -> Result<()> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let rows_affected = conn.execute("DELETE FROM daily_notes WHERE date = ?1", params![date_str])?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Daily note not found for date: {}", date_str)));
        }
        
        Ok(())
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
    fn test_create_daily_note() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Daily Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let date = NaiveDate::from_ymd_opt(2024, 10, 7).unwrap();
        let daily_note = DailyNote::new(date, note.id.clone());
        
        DailyNoteRepository::create(&conn, &daily_note).unwrap();
        
        let retrieved = DailyNoteRepository::get_by_date(&conn, date).unwrap();
        assert_eq!(retrieved.note_id, note.id);
    }

    #[test]
    fn test_get_or_create() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Daily Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let date = NaiveDate::from_ymd_opt(2024, 10, 7).unwrap();
        
        let daily_note1 = DailyNoteRepository::get_or_create(&conn, date, note.id.clone()).unwrap();
        let daily_note2 = DailyNoteRepository::get_or_create(&conn, date, "different-id".to_string()).unwrap();
        
        // Should return the existing one, not create a new one
        assert_eq!(daily_note1.note_id, daily_note2.note_id);
    }

    #[test]
    fn test_get_all() {
        let (_dir, conn) = setup_test_db();
        
        let note1 = Note::new("Note 1".to_string());
        let note2 = Note::new("Note 2".to_string());
        NoteRepository::create(&conn, &note1).unwrap();
        NoteRepository::create(&conn, &note2).unwrap();
        
        let date1 = NaiveDate::from_ymd_opt(2024, 10, 7).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 10, 8).unwrap();
        
        let daily1 = DailyNote::new(date1, note1.id.clone());
        let daily2 = DailyNote::new(date2, note2.id.clone());
        
        DailyNoteRepository::create(&conn, &daily1).unwrap();
        DailyNoteRepository::create(&conn, &daily2).unwrap();
        
        let all = DailyNoteRepository::get_all(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }
}

