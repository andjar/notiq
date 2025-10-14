use anyhow::Result;
use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, Event as CEvent, KeyEventKind};
use std::time::Duration;
use notiq_core::storage::NoteRepository;
use crate::app::App;

fn parse_keybinding(kb: &str) -> (KeyCode, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut key_code_str = kb;

    if let Some(parts) = kb.rsplit_once('-') {
        let mod_str = parts.0;
        key_code_str = parts.1;

        for m in mod_str.split('-') {
            match m.to_lowercase().as_str() {
                "ctrl" => modifiers.insert(KeyModifiers::CONTROL),
                "alt" => modifiers.insert(KeyModifiers::ALT),
                "shift" => modifiers.insert(KeyModifiers::SHIFT),
                _ => {}
            }
        }
    }

    let key_code = match key_code_str {
        "enter" => KeyCode::Enter,
        "esc" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => KeyCode::Null,
    };

    (key_code, modifiers)
}


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

    // Search results take precedence
    if !app.search_results.is_empty() {
        handle_search_results_input(key, app);
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
                if app.search_query.starts_with('#') {
                    let name = app.search_query.trim_start_matches('#').trim().to_string();
                    if !name.is_empty() { let _ = app.set_tag_filter(name); }
                    app.close_search();
                } else {
                    let _ = app.perform_search();
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

    let keymap = &app.config.keymap;

    let (quit_kc, quit_km) = parse_keybinding(&keymap.quit);
    let (toggle_sidebar_kc, toggle_sidebar_km) = parse_keybinding(&keymap.toggle_sidebar);
    let (open_page_switcher_kc, open_page_switcher_km) = parse_keybinding(&keymap.open_page_switcher);
    let (create_new_page_kc, create_new_page_km) = parse_keybinding(&keymap.create_new_page);
    let (delete_current_page_kc, delete_current_page_km) = parse_keybinding(&keymap.delete_current_page);
    let (toggle_favorite_kc, toggle_favorite_km) = parse_keybinding(&keymap.toggle_favorite);
    let (open_logbook_kc, open_logbook_km) = parse_keybinding(&keymap.open_logbook);
    let (export_kc, export_km) = parse_keybinding(&keymap.export);
    let (attach_kc, attach_km) = parse_keybinding(&keymap.attach);
    let (open_attachment_kc, open_attachment_km) = parse_keybinding(&keymap.open_attachment);
    let (attachments_select_up_kc, attachments_select_up_km) = parse_keybinding(&keymap.attachments_select_up);
    let (attachments_select_down_kc, attachments_select_down_km) = parse_keybinding(&keymap.attachments_select_down);
    let (sidebar_select_up_kc, sidebar_select_up_km) = parse_keybinding(&keymap.sidebar_select_up);
    let (sidebar_select_down_kc, sidebar_select_down_km) = parse_keybinding(&keymap.sidebar_select_down);
    let (sidebar_activate_kc, sidebar_activate_km) = parse_keybinding(&keymap.sidebar_activate);
    let (move_up_kc, move_up_km) = parse_keybinding(&keymap.move_up);
    let (move_down_kc, move_down_km) = parse_keybinding(&keymap.move_down);
    let (cursor_up_kc, cursor_up_km) = parse_keybinding(&keymap.cursor_up);
    let (cursor_down_kc, cursor_down_km) = parse_keybinding(&keymap.cursor_down);
    let (expand_kc, expand_km) = parse_keybinding(&keymap.expand);
    let (collapse_kc, collapse_km) = parse_keybinding(&keymap.collapse);
    let (start_editing_kc, start_editing_km) = parse_keybinding(&keymap.start_editing);
    let (create_sibling_kc, create_sibling_km) = parse_keybinding(&keymap.create_sibling);
    let (initiate_delete_kc, initiate_delete_km) = parse_keybinding(&keymap.initiate_delete);
    let (task_overview_kc, task_overview_km) = parse_keybinding(&keymap.task_overview);
    let (clear_tag_filter_kc, clear_tag_filter_km) = parse_keybinding(&keymap.clear_tag_filter);
    let (paste_kc, paste_km) = parse_keybinding(&keymap.paste);
    let (rename_page_kc, rename_page_km) = parse_keybinding(&keymap.rename_page);
    let (help_kc, help_km) = parse_keybinding(&keymap.help);
    let (create_quote_block_kc, create_quote_block_km) = parse_keybinding(&keymap.create_quote_block);
    let (create_code_block_kc, create_code_block_km) = parse_keybinding(&keymap.create_code_block);
    let (toggle_task_kc, toggle_task_km) = parse_keybinding(&keymap.toggle_task);
    let (search_kc, search_km) = parse_keybinding(&keymap.search);

    // --- Global key handlers (not in a specific mode) ---
    match key.code {
        // Calendar interactions are not configurable for now
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_move_day(-1),
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_move_day(1),
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_move_week(-1),
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_move_week(1),
        KeyCode::PageUp if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_prev_month(),
        KeyCode::PageDown if key.modifiers.contains(KeyModifiers::SHIFT) => app.calendar_next_month(),
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            let _ = app.open_selected_daily_note();
        }

        kc if kc == toggle_task_kc && key.modifiers == toggle_task_km => {
            let _ = app.toggle_selected_task();
        }
        kc if kc == search_kc && key.modifiers == search_km => app.open_search(),
        kc if kc == quit_kc && key.modifiers == quit_km => app.quit(),
        kc if kc == toggle_sidebar_kc && key.modifiers == toggle_sidebar_km => app.toggle_sidebar(),
        kc if kc == open_page_switcher_kc && key.modifiers == open_page_switcher_km => {
            let _ = app.open_page_switcher();
        }
        kc if kc == create_new_page_kc && key.modifiers == create_new_page_km => {
            let _ = app.create_new_page();
        }
        kc if kc == delete_current_page_kc && key.modifiers == delete_current_page_km => {
            let _ = app.delete_current_page();
        }
        kc if kc == toggle_favorite_kc && key.modifiers == toggle_favorite_km => {
            let _ = app.toggle_favorite_current();
        }
        kc if kc == open_logbook_kc && key.modifiers == open_logbook_km => {
            let _ = app.open_logbook_for_selected();
        }
        KeyCode::Esc => {
            if app.logbook_open {
                app.close_logbook();
            }
        }
        kc if kc == export_kc && key.modifiers == export_km => {
            let out = std::path::PathBuf::from("export");
            let _ = app.export_markdown(&out);
        }
        kc if kc == attach_kc && key.modifiers == attach_km => {
            app.open_attachments_overlay();
        }
        kc if kc == open_attachment_kc && key.modifiers == open_attachment_km => {
            let _ = app.open_selected_attachment();
        }
        kc if kc == attachments_select_up_kc && key.modifiers == attachments_select_up_km => app.attachments_select_up(),
        kc if kc == attachments_select_down_kc && key.modifiers == attachments_select_down_km => app.attachments_select_down(),
        kc if kc == sidebar_select_up_kc && key.modifiers == sidebar_select_up_km => app.sidebar_select_up(),
        kc if kc == sidebar_select_down_kc && key.modifiers == sidebar_select_down_km => app.sidebar_select_down(),
        kc if kc == sidebar_activate_kc && key.modifiers == sidebar_activate_km => {
            let _ = app.sidebar_activate_selected();
        }
        kc if kc == move_up_kc && key.modifiers == move_up_km => {
            let _ = app.move_selected_up();
        }
        kc if kc == move_down_kc && key.modifiers == move_down_km => {
            let _ = app.move_selected_down();
        }
        kc if kc == cursor_up_kc && key.modifiers == cursor_up_km => app.move_cursor_up(),
        kc if kc == cursor_down_kc && key.modifiers == cursor_down_km => app.move_cursor_down(),
        kc if kc == collapse_kc && key.modifiers == collapse_km => app.toggle_selected_expand_collapse(Some(false)),
        kc if kc == expand_kc && key.modifiers == expand_km => app.toggle_selected_expand_collapse(Some(true)),
        kc if kc == start_editing_kc && key.modifiers == start_editing_km => app.start_editing(),
        kc if kc == create_sibling_kc && key.modifiers == create_sibling_km => {
            let _ = app.create_sibling_below();
        }
        kc if kc == initiate_delete_kc && key.modifiers == initiate_delete_km => {
            app.initiate_delete();
        }
        kc if kc == task_overview_kc && key.modifiers == task_overview_km => {
            app.open_task_overview();
        }
        kc if kc == clear_tag_filter_kc && key.modifiers == clear_tag_filter_km => {
            let _ = app.clear_tag_filter();
        }
        kc if kc == paste_kc && key.modifiers == paste_km => {
            let _ = app.paste_from_clipboard();
        }
        kc if kc == rename_page_kc && key.modifiers == rename_page_km => {
            app.start_renaming_page();
        }
        kc if kc == help_kc && key.modifiers == help_km => {
            app.open_help();
        }
        kc if kc == create_quote_block_kc && key.modifiers == create_quote_block_km => {
            let _ = app.create_quote_block();
        }
        kc if kc == create_code_block_kc && key.modifiers == create_code_block_km => {
            let _ = app.create_code_block();
        }
        _ => {}
    }
}

fn handle_search_results_input(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.search_results.clear();
            app.search_selection = 0;
        }
        KeyCode::Up => app.search_results_up(),
        KeyCode::Down => app.search_results_down(),
        KeyCode::Enter => {
            let _ = app.search_results_select();
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
                    let calendar_h = 9u16;
                    let tags_h = 10u16;
                    let favorites_h = 6u16;

                    // Calendar area
                    if y >= content_top && y < content_top + calendar_h {
                        let calendar_y = y - content_top;
                        if calendar_y >= 3 && calendar_y <= 8 {
                            let day_row = (calendar_y - 3) as usize;
                            let day_col = ((x as i32 - 1) / 3) as usize;
                            if day_col < 7 {
                                let _ = app.calendar_click_day(day_row, day_col);
                            }
                        }
                    } 
                    // Tags area (no action for now)
                    else if y < content_top + calendar_h + tags_h {
                        //
                    }
                    // Favorites area
                    else if y < content_top + calendar_h + tags_h + favorites_h {
                        let row_in_list = (y - (content_top + calendar_h + tags_h)) as usize;
                        if row_in_list < app.favorites.len() {
                            let _ = app.select_favorite_by_index(row_in_list);
                        }
                    }
                    // Pages list area
                    else {
                        let row_in_list = (y - (content_top + calendar_h + tags_h + favorites_h)) as usize;
                        if row_in_list < app.notes.len() {
                            let idx = row_in_list;
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

