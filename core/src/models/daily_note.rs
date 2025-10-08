use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyNote {
    pub date: NaiveDate,
    pub note_id: String,
}

impl DailyNote {
    /// Create a new daily note
    pub fn new(date: NaiveDate, note_id: String) -> Self {
        Self { date, note_id }
    }

    /// Format the date as YYYY-MM-DD
    pub fn date_string(&self) -> String {
        self.date.format("%Y-%m-%d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_note_creation() {
        let date = NaiveDate::from_ymd_opt(2024, 10, 7).unwrap();
        let daily_note = DailyNote::new(date, "note-1".to_string());
        assert_eq!(daily_note.date, date);
        assert_eq!(daily_note.date_string(), "2024-10-07");
    }
}

