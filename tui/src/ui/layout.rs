use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use super::{render_header, render_outline, render_status_bar, render_page_switcher, render_search_overlay, render_sidebar_tags_and_pages, render_backlinks_panel, render_attachments_panel, render_attach_overlay, render_logbook, render_delete_confirmation, render_autocomplete, render_task_overview, render_rename_page_overlay, render_help_screen};

/// Render the complete UI
pub fn render(frame: &mut Frame, app: &mut App) {
    app.link_locations.clear();
    let size = frame.size();

    // Create main layout: header, content, status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Content
            Constraint::Length(1),  // Status bar
        ])
        .split(size);

    // Render components
    render_header(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);

    // Overlays (drawn last)
    if app.page_switcher_open {
        render_page_switcher(frame, app, size);
    }
    if app.search_open {
        render_search_overlay(frame, app, size);
    }
    if app.attach_overlay_open {
        render_attach_overlay(frame, app, size);
    }
    if app.logbook_open {
        render_logbook(frame, app, size);
    }
    if app.confirming_delete {
        render_delete_confirmation(frame, app, size);
    }
    if app.task_overview_open {
        render_task_overview(frame, app, size);
    }
    if app.is_renaming_page {
        render_rename_page_overlay(frame, app, size);
    }
    if app.help_open {
        render_help_screen(frame, app, size);
    }
    // Autocomplete is rendered last (on top of everything)
    if app.autocomplete_open {
        render_autocomplete(frame, app, size);
    }
}

/// Render the main content area (will have sidebar + outliner in future)
fn render_content(frame: &mut Frame, app: &mut App, area: Rect) {
    // Phase 4: Split content into sidebar and outline
    // Dynamic layout: optional sidebar; split backlinks/attachments vertically
    if app.show_sidebar {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30), // Sidebar
                Constraint::Min(0),     // Outline
                Constraint::Length(30), // Right column
            ])
            .split(area);

        render_sidebar_tags_and_pages(frame, app, main_chunks[0]);
        render_outline(frame, app, main_chunks[1]);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Backlinks upper
                Constraint::Percentage(40), // Attachments lower
            ])
            .split(main_chunks[2]);
        render_backlinks_panel(frame, app, right_chunks[0]);
        render_attachments_panel(frame, app, right_chunks[1]);
    } else {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),     // Outline only
                Constraint::Length(30), // Right column
            ])
            .split(area);
        render_outline(frame, app, main_chunks[0]);
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ])
            .split(main_chunks[1]);
        render_backlinks_panel(frame, app, right_chunks[0]);
        render_attachments_panel(frame, app, right_chunks[1]);
    }
}

