use crate::{Error, Result};
use rusqlite::{Connection as SqliteConnection};
use std::path::{Path, PathBuf};

pub type Connection = SqliteConnection;

/// Database manager for the notiq application
pub struct Database {
    db_path: PathBuf,
}

impl Database {
    /// Create a new database manager
    pub fn new<P: AsRef<Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    /// Get a connection to the database
    pub fn connect(&self) -> Result<Connection> {
        let conn = SqliteConnection::open(&self.db_path)?;
        
        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        
        Ok(conn)
    }

    /// Create a new database and initialize it with the schema
    pub fn create(&self) -> Result<Connection> {
        // Ensure parent directory exists
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = SqliteConnection::open(&self.db_path)?;
        
        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        
        // Initialize schema
        self.initialize_schema(&conn)?;
        
        Ok(conn)
    }

    /// Initialize the database schema
    fn initialize_schema(&self, conn: &Connection) -> Result<()> {
        let schema = include_str!("../../../core/schema.sql");
        conn.execute_batch(schema)?;
        Ok(())
    }

    /// Check if the database exists
    pub fn exists(&self) -> bool {
        self.db_path.exists()
    }

    /// Get or create a database connection
    pub fn get_or_create(&self) -> Result<Connection> {
        if self.exists() {
            self.connect()
        } else {
            self.create()
        }
    }

    /// Get the database path
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Run a migration (for future schema updates)
    pub fn migrate(&self, _conn: &Connection, _from_version: i32, _to_version: i32) -> Result<()> {
        // Placeholder for future migrations
        Ok(())
    }

    /// Get the current schema version
    pub fn get_schema_version(&self, conn: &Connection) -> Result<i32> {
        let version: String = conn.query_row(
            "SELECT value FROM metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        
        version.parse::<i32>()
            .map_err(|_| Error::InvalidInput("Invalid schema version".to_string()))
    }

    /// Backup the database
    pub fn backup<P: AsRef<Path>>(&self, backup_path: P) -> Result<()> {
        std::fs::copy(&self.db_path, backup_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = Database::new(&db_path);
        assert!(!db.exists());
        
        let conn = db.create().unwrap();
        assert!(db.exists());
        
        // Verify schema was initialized
        let version = db.get_schema_version(&conn).unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_database_connect() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = Database::new(&db_path);
        db.create().unwrap();
        
        // Should be able to connect to existing database
        let _conn = db.connect().unwrap();
    }

    #[test]
    fn test_get_or_create() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let db = Database::new(&db_path);
        
        // First call should create
        let _conn1 = db.get_or_create().unwrap();
        assert!(db.exists());
        
        // Second call should connect
        let _conn2 = db.get_or_create().unwrap();
    }

    #[test]
    fn test_backup() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let backup_path = dir.path().join("backup.db");
        
        let db = Database::new(&db_path);
        db.create().unwrap();
        
        db.backup(&backup_path).unwrap();
        assert!(backup_path.exists());
    }
}

