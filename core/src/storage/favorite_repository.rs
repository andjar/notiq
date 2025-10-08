use crate::models::{Favorite, datetime_to_timestamp, timestamp_to_datetime};
use crate::{Error, Result};
use rusqlite::{Connection, params};

pub struct FavoriteRepository;

impl FavoriteRepository {
    /// Add a note to favorites
    pub fn create(conn: &Connection, favorite: &Favorite) -> Result<()> {
        conn.execute(
            "INSERT INTO favorites (note_id, position, created_at) VALUES (?1, ?2, ?3)",
            params![
                favorite.note_id,
                favorite.position,
                datetime_to_timestamp(&favorite.created_at),
            ],
        )?;
        
        Ok(())
    }

    /// Check if a note is favorited
    pub fn is_favorited(conn: &Connection, note_id: &str) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM favorites WHERE note_id = ?1",
            params![note_id],
            |row| row.get(0),
        )?;
        
        Ok(count > 0)
    }

    /// Get all favorites ordered by position
    pub fn get_all(conn: &Connection) -> Result<Vec<Favorite>> {
        let mut stmt = conn.prepare(
            "SELECT note_id, position, created_at FROM favorites ORDER BY position"
        )?;
        
        let favorites = stmt.query_map([], |row| {
            Ok(Favorite {
                note_id: row.get(0)?,
                position: row.get(1)?,
                created_at: timestamp_to_datetime(row.get(2)?),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(favorites)
    }

    /// Remove a note from favorites
    pub fn delete(conn: &Connection, note_id: &str) -> Result<()> {
        let rows_affected = conn.execute("DELETE FROM favorites WHERE note_id = ?1", params![note_id])?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Favorite not found: {}", note_id)));
        }
        
        Ok(())
    }

    /// Reorder favorites
    pub fn update_position(conn: &Connection, note_id: &str, new_position: i32) -> Result<()> {
        let rows_affected = conn.execute(
            "UPDATE favorites SET position = ?1 WHERE note_id = ?2",
            params![new_position, note_id],
        )?;
        
        if rows_affected == 0 {
            return Err(Error::NotFound(format!("Favorite not found: {}", note_id)));
        }
        
        Ok(())
    }

    /// Get the next available position
    pub fn get_next_position(conn: &Connection) -> Result<i32> {
        let max_position: Option<i32> = conn.query_row(
            "SELECT MAX(position) FROM favorites",
            [],
            |row| row.get(0),
        )?;
        
        Ok(max_position.map(|p| p + 1).unwrap_or(0))
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
    fn test_add_favorite() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let favorite = Favorite::new(note.id.clone(), 0);
        FavoriteRepository::create(&conn, &favorite).unwrap();
        
        assert!(FavoriteRepository::is_favorited(&conn, &note.id).unwrap());
    }

    #[test]
    fn test_get_all_favorites() {
        let (_dir, conn) = setup_test_db();
        
        let note1 = Note::new("Note 1".to_string());
        let note2 = Note::new("Note 2".to_string());
        NoteRepository::create(&conn, &note1).unwrap();
        NoteRepository::create(&conn, &note2).unwrap();
        
        let fav1 = Favorite::new(note1.id.clone(), 0);
        let fav2 = Favorite::new(note2.id.clone(), 1);
        
        FavoriteRepository::create(&conn, &fav1).unwrap();
        FavoriteRepository::create(&conn, &fav2).unwrap();
        
        let favorites = FavoriteRepository::get_all(&conn).unwrap();
        assert_eq!(favorites.len(), 2);
        assert_eq!(favorites[0].position, 0);
        assert_eq!(favorites[1].position, 1);
    }

    #[test]
    fn test_remove_favorite() {
        let (_dir, conn) = setup_test_db();
        
        let note = Note::new("Test Note".to_string());
        NoteRepository::create(&conn, &note).unwrap();
        
        let favorite = Favorite::new(note.id.clone(), 0);
        FavoriteRepository::create(&conn, &favorite).unwrap();
        
        FavoriteRepository::delete(&conn, &note.id).unwrap();
        
        assert!(!FavoriteRepository::is_favorited(&conn, &note.id).unwrap());
    }

    #[test]
    fn test_get_next_position() {
        let (_dir, conn) = setup_test_db();
        
        assert_eq!(FavoriteRepository::get_next_position(&conn).unwrap(), 0);
        
        let note1 = Note::new("Note 1".to_string());
        NoteRepository::create(&conn, &note1).unwrap();
        let fav1 = Favorite::new(note1.id.clone(), 0);
        FavoriteRepository::create(&conn, &fav1).unwrap();
        
        assert_eq!(FavoriteRepository::get_next_position(&conn).unwrap(), 1);
    }
}

