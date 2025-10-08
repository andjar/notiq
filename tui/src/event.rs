use anyhow::Result;
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, Event as CEvent, KeyEventKind};
use std::time::Duration;
use notiq_core::storage::NoteRepository;

/// Terminal events
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Terminal tick event
    Tick,
    /// Mouse event
    Mouse(MouseEvent),
}

/// Event handler for the terminal
pub struct EventHandler {
    /// Tick rate in milliseconds
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate_ms: u64) -> Self {
        Self {
            tick_rate: Duration::from_millis(tick_rate_ms),
        }
    }

    /// Poll for the next event
    pub fn next(&self) -> Result<Event> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                CEvent::Key(key) => return Ok(Event::Key(key)),
                CEvent::Mouse(m) => return Ok(Event::Mouse(m)),
                _ => {}
            }
        }
        Ok(Event::Tick)
    }
}

/// Handle key events for the application
pub fn handle_key_event(key: KeyEvent, app: &mut crate::app::App) {
    // On Windows, crossterm reports both key press and release events.
    // We only want to handle press events to avoid duplicates.
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Attach overlay takes precedence
    if app.attach_overlay_open {
        match key.code {
            KeyCode::Esc => app.close_attachments_overlay(),
            KeyCode::Enter => { let _ = app.confirm_attach(); },
            KeyCode::Backspace => { app.backspace_attach_input(); },
            KeyCode::Char(c) => { 
                if !key.modifiers.contains(KeyModifiers::CONTROL) { 
                    app.update_attach_input(c); 
                } 
            },
            _ => {}
        }
        return;
    }
    
    // When search/autocomplete is open, handle that first
    if app.search_open || app.autocomplete_open {
        if app.autocomplete_open {
            handle_autocomplete_input(key, app);
            return;
        }
        match key.code {
            KeyCode::Esc => app.close_search(),
            KeyCode::Enter => {
                // If query starts with #, treat as tag filter
                if app.search_query.starts_with('#') {
                    let name = app.search_query.trim_start_matches('#').trim().to_string();
                    if !name.is_empty() { let _ = app.set_tag_filter(name); }
                    app.close_search();
                }
            }
            KeyCode::Backspace => { app.backspace_search_query(); },
            KeyCode::Char(c) => { 
                if !key.modifiers.contains(KeyModifiers::CONTROL) { 
                    app.update_search_query(c); 
                } 
            },
            _ => {}
        }
        return;
    }

    // Help screen takes precedence
    if app.help_open {
        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => app.close_help(),
            _ => {}
        }
        return;
    }

    // Page rename overlay takes precedence
    if app.is_renaming_page {
        match key.code {
            KeyCode::Esc => app.cancel_page_rename(),
            KeyCode::Enter => { let _ = app.commit_page_rename(); },
            KeyCode::Backspace => { app.page_title_buffer.pop(); },
            KeyCode::Char(c) => {
                // Allow AltGr combinations (CONTROL+ALT) for special characters
                if !key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
                    app.page_title_buffer.push(c);
                }
            },
            _ => {}
        }
        return;
    }
    
    if app.confirming_delete {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => { let _ = app.confirm_delete(); }
            KeyCode::Char('n') | KeyCode::Esc => { app.cancel_delete(); }
            _ => {}
        }
        return;
    }

    // When page switcher is open, handle its own controls first
    if app.page_switcher_open {
        match key.code {
            KeyCode::Esc => app.close_page_switcher(),
            KeyCode::Up => app.page_switcher_up(),
            KeyCode::Down => app.page_switcher_down(),
            KeyCode::Enter => { let _ = app.page_switcher_activate(); },
            KeyCode::Backspace => { app.page_filter.pop(); },
            KeyCode::Char(c) => { 
                if !key.modifiers.contains(KeyModifiers::CONTROL) { 
                    app.page_filter.push(c); 
                } 
            },
            _ => {}
        }
        return;
    }

    // Task overview mode
    if app.task_overview_open {
        // Task overview has its own input handler now
        handle_task_overview_input(key, app);
        return;
    }

    // If in edit mode, handle editing-specific keys and return
    if app.is_editing {
        handle_editing_input(key, app);
        return;
    }

    // --- Global key handlers (not in a specific mode) ---
    match key.code {
        // Calendar interactions (Shift-modified first to avoid unreachable patterns)
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_move_day(-1),
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_move_day(1),
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_move_week(-1),
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_move_week(1),
        KeyCode::PageUp if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_prev_month(),
        KeyCode::PageDown if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_next_month(),
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            let _ = app.open_selected_daily_note();
        }
        // Task toggle
        KeyCode::Char('x') | KeyCode::Char(' ') => {
            let _ = app.toggle_selected_task();
        }
        // Search toggle
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('q') | KeyCode::Char('Q') if !key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),
        // Toggle sidebar
        KeyCode::Char('b') | KeyCode::Char('B') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_sidebar();
        }
        // Page management shortcuts (Phase 4)
        KeyCode::Char('p') | KeyCode::Char('P') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let _ = app.open_page_switcher();
        }
        KeyCode::Char('n') | KeyCode::Char('N') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let _ = app.create_new_page();
        }
        KeyCode::Char('d') | KeyCode::Char('D') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let _ = app.delete_current_page();
        }
        // Favorites toggle
        KeyCode::Char('f') | KeyCode::Char('F') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let _ = app.toggle_favorite_current();
        }
        // Logbook for selected task
        KeyCode::Char('l') | KeyCode::Char('L') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let _ = app.open_logbook_for_selected();
        }
        KeyCode::Esc => {
            if app.logbook_open {
                app.close_logbook();
            }
        }
        // Export
        KeyCode::Char('e') | KeyCode::Char('E') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let out = std::path::PathBuf::from("export");
            let _ = app.export_markdown(&out);
        }
        // Attachments (Phase 7)
        KeyCode::Char('a') | KeyCode::Char('A') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.open_attachments_overlay();
        }
        KeyCode::Char('o') | KeyCode::Char('O') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let _ = app.open_selected_attachment();
        }
        KeyCode::Char('[') => app.attachments_select_up(), // navigate attachments up
        KeyCode::Char(']') => app.attachments_select_down(), // navigate attachments down
        // Sidebar navigation (use PageUp/PageDown to move selection; Enter to open)
        KeyCode::PageUp => app.sidebar_select_up(),
        KeyCode::PageDown => app.sidebar_select_down(),
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
            let _ = app.sidebar_activate_selected();
        }
        // Move/reorder with Alt, plain navigation otherwise (order of patterns matters)
        KeyCode::Up if key.modifiers.contains(KeyModifiers::ALT) => {
            let _ = app.move_selected_up();
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
            let _ = app.move_selected_down();
        }
        KeyCode::Up => app.move_cursor_up(),
        KeyCode::Down => app.move_cursor_down(),
        // Expand/Collapse
        KeyCode::Left => app.toggle_selected_expand_collapse(Some(false)),
        KeyCode::Right => app.toggle_selected_expand_collapse(Some(true)),
        // Edit mode controls (generic Enter after Shift+Enter)
        KeyCode::Enter => app.start_editing(),
        KeyCode::Char(ch) => {
            if ch == 'n' {
                let _ = app.create_sibling_below();
            } else if ch == 'd' {
                app.initiate_delete();
            } else if ch == 't' && key.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+T for task overview OR clear tag filter
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.open_task_overview();
                } else {
                    let _ = app.clear_tag_filter();
                }
            } else if ch == 'v' && key.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+V paste image from clipboard
                let _ = app.paste_from_clipboard();
            } else if ch == 'r' && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.start_renaming_page();
            } else if ch == 'h' && !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.open_help();
            } else if ch == 'q' && key.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+Q for quote block
                let _ = app.create_quote_block();
            } else if ch == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+C for code block
                let _ = app.create_code_block();
            }
        }
        // CRUD via non-char
        KeyCode::Insert => {
            let _ = app.create_sibling_below();
        }
        KeyCode::Delete => app.initiate_delete(),
        // Indent / Outdent
        KeyCode::Tab => {
            let _ = app.indent_selected();
        }
        KeyCode::BackTab => {
            let _ = app.outdent_selected();
        }
        _ => {}
    }
}

/// Handle key events when in editing mode
fn handle_editing_input(key: KeyEvent, app: &mut crate::app::App) {
    match key.code {
        KeyCode::Enter => {
            let _ = app.commit_edit();
        }
        KeyCode::Esc => app.cancel_edit(),
        KeyCode::Backspace => {
            if app.edit_cursor_position > 0 {
                let current_pos = app.edit_cursor_position;
                let from = app.edit_buffer.char_indices().map(|(i, _)| i).nth(current_pos - 1);
                if let Some(from) = from {
                    app.edit_buffer.remove(from);
                    app.edit_cursor_position -= 1;
                }
            }
            // Check for autocomplete trigger after deletion
            app.check_autocomplete_trigger();
        }
        KeyCode::Left => {
            if app.edit_cursor_position > 0 {
                app.edit_cursor_position -= 1;
            }
        }
        KeyCode::Right => {
            if app.edit_cursor_position < app.edit_buffer.chars().count() {
                app.edit_cursor_position += 1;
            }
        }
        KeyCode::Home => {
            app.edit_cursor_position = 0;
        }
        KeyCode::End => {
            app.edit_cursor_position = app.edit_buffer.chars().count();
        }
        KeyCode::Char(c) => {
            // Check for modifiers to avoid capturing Ctrl+C, etc.
            // Allow AltGr combinations (CONTROL+ALT) for special characters
            if !key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
                let current_pos = app.edit_cursor_position;
                let byte_pos = app.edit_buffer.char_indices().map(|(i, _)| i).nth(current_pos).unwrap_or(app.edit_buffer.len());
                app.edit_buffer.insert(byte_pos, c);
                app.edit_cursor_position += 1;
                // Check if we should trigger autocomplete
                app.check_autocomplete_trigger();
            } else if c == 'v' && key.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+V paste from clipboard
                let _ = app.paste_from_clipboard();
            }
        }
        _ => {}
    }
}

/// Handle key events when the task overview is open
fn handle_task_overview_input(key: KeyEvent, app: &mut crate::app::App) {
    match key.code {
        KeyCode::Esc => app.close_task_overview(),
        KeyCode::Up => app.task_overview_up(),
        KeyCode::Down => app.task_overview_down(),
        KeyCode::Enter => {
            let _ = app.task_overview_goto_selected();
        }
        KeyCode::Char('x') | KeyCode::Char(' ') => {
            let _ = app.task_overview_toggle_selected();
        }
        _ => {}
    }
}

/// Handle autocomplete input
fn handle_autocomplete_input(key: KeyEvent, app: &mut crate::app::App) {
    match key.code {
        KeyCode::Esc => app.close_autocomplete(),
        KeyCode::Up => app.autocomplete_up(),
        KeyCode::Down => app.autocomplete_down(),
        KeyCode::Enter | KeyCode::Tab => {
            let _ = app.autocomplete_select();
        }
        KeyCode::Backspace => {
            app.edit_buffer.pop();
            app.check_autocomplete_trigger();
        }
        KeyCode::Char(c) => {
            // Allow AltGr combinations (CONTROL+ALT) for special characters
            if !key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
                app.edit_buffer.push(c);
                app.check_autocomplete_trigger();
            }
        }
        _ => {}
    }
}

/// Handle mouse events: basic clicks on sidebar pages, outline selection, and calendar
pub fn handle_mouse_event(mouse: MouseEvent, app: &mut crate::app::App, _size: ratatui::prelude::Rect) {
    match mouse.kind {
        MouseEventKind::Down(_) => {
            // Check for link clicks first. Need to clone to avoid borrow checker issues.
            let locations = app.link_locations.clone();
            for (rect, target_title) in &locations {
                if rect.contains(ratatui::layout::Position::new(mouse.column, mouse.row)) {
                    if let Ok(target_note) = NoteRepository::get_by_title_exact(&app.db_connection, target_title) {
                        if app.load_note(&target_note.id).is_ok() {
                            return; // Click handled
                        }
                    }
                }
            }

            // Very simple hit-testing based on layout from layout.rs
            // Header: 3 rows, Status: 1 row -> content between
            let y = mouse.row;
            let x = mouse.column;
            let size = _size;
            if y >= 3 && y < size.height.saturating_sub(1) {
                // Content area
                let content_top = 3u16;
                let content_left_sidebar_w = if app.show_sidebar { 30u16 } else { 0u16 };
                let backlinks_w = 30u16;
                let attachments_w = 30u16;
                let total_w = size.width;
                
                // Sidebar click
                if app.show_sidebar && x < content_left_sidebar_w {
                    // Calendar area: rows 3-12 (9 rows total including border)
                    if y >= content_top && y < content_top + 9 {
                        let calendar_y = y - content_top;
                        // Skip title (row 0, 1) and weekday header (row 2)
                        if calendar_y >= 3 && calendar_y <= 8 {
                            let day_row = (calendar_y - 3) as usize;
                            // Each day cell is 3 chars wide (2 digits + space)
                            let day_col = ((x as i32 - 1) / 3) as usize;
                            if day_col < 7 {
                                let _ = app.calendar_click_day(day_row, day_col);
                            }
                        }
                    }
                    // Pages list area: after calendar(9) + tags(10) + favorites(6) + border(1)
                    else {
                        let row_in_list = (y - content_top).saturating_sub(9 + 10 + 6 + 1);
                        if row_in_list < app.notes.len() as u16 {
                            let idx = row_in_list as usize;
                            let _ = app.select_page_by_index(idx);
                        }
                    }
                } else if x >= content_left_sidebar_w && x < total_w.saturating_sub(backlinks_w + attachments_w) {
                    // Outline area: map y to visible index
                    let list_row = (y - content_top).saturating_sub(1) as usize; // border title offset
                    let target_index = app.scroll_offset + list_row;
                    let visible_len = app.get_visible_nodes().len();
                    if target_index < visible_len {
                        app.cursor_position = target_index;
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => { app.move_cursor_up(); },
        MouseEventKind::ScrollDown => { app.move_cursor_down(); },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_handler_creation() {
        let handler = EventHandler::new(250);
        assert_eq!(handler.tick_rate, Duration::from_millis(250));
    }
}

