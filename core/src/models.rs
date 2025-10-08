mod note;
mod outline_node;
mod tag;
mod link;
mod attachment;
mod daily_note;
mod favorite;
mod task_log;

pub use note::Note;
pub use outline_node::{OutlineNode, TaskPriority, BlockType};
pub use tag::Tag;
pub use link::{Link, LinkType};
pub use attachment::Attachment;
pub use daily_note::DailyNote;
pub use favorite::Favorite;
pub use task_log::{TaskStatusLog, TaskStatus};

use chrono::{DateTime, Utc};

/// Convert Unix timestamp (seconds) to DateTime<Utc>
pub fn timestamp_to_datetime(timestamp: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(timestamp, 0).unwrap_or_default()
}

/// Convert DateTime<Utc> to Unix timestamp (seconds)
pub fn datetime_to_timestamp(datetime: &DateTime<Utc>) -> i64 {
    datetime.timestamp()
}

