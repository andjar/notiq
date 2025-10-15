pub mod app;
pub mod event;
pub mod ui;
pub mod config;

// Re-export commonly used types
pub use app::App;
pub use event::{Event, EventHandler};
