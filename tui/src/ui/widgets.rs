use crate::app::{App, TreeNode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use notiq_core::storage::{TagRepository, LinkRepository, NoteRepository, NodeRepository};
use chrono::{Datelike, NaiveDate, Weekday};
use regex::Regex;

/// Render the header with title and key hints
pub fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(note) = &app.current_note {
        format!(" 📝 {} ", note.title)
    } else {
        " Notiq ".to_string()
    };

    let key_hints = if app.is_editing {
        " [Enter:Save] [Esc:Cancel] [Typing...] "
    } else if app.page_switcher_open {
        " [Esc:Close] [↑/↓:Select] [Enter:Open] [Type to filter] "
    } else if app.search_open {
        " [Esc:Close] [Type to search] [Backspace:Delete] "
    } else if app.logbook_open {
        " [Esc:Close Logbook] "
    } else {
        " [q:Quit] [h:Help] [↑/↓:Move] [←/→:Expand] [Enter:Edit] [n:New] [d:Del] [x:Task] [Tab:Indent] [/:Search] [Ctrl+P:Pages] [Ctrl+F:Fav] [Ctrl+L:Logbook] [Ctrl+E:Export] "
    };

    let header_spans = vec![
        Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(key_hints, Style::default().fg(Color::DarkGray)),
    ];

    let header = Paragraph::new(Line::from(header_spans))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    frame.render_widget(header, area);
}

/// Render the outline view
pub fn render_outline(frame: &mut Frame, app: &mut App, area: Rect) {
    let visible_nodes = app.get_visible_nodes();

    if visible_nodes.is_empty() {
        let empty_message = Paragraph::new("This page is empty. Press 'n' to add a node or Ctrl+N to create a new page.")
            .block(Block::default().borders(Borders::ALL).title(" Outline "))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty_message, area);
        return;
    }

    // Build lines for each visible node
    let mut lines: Vec<Line> = Vec::new();

    for (i, tree_node) in visible_nodes.iter().enumerate().skip(app.scroll_offset) {
        // Check if this is the node being edited
        let is_editing_this = app.is_editing && i == app.cursor_position;
        
        let mut line = if is_editing_this {
            // Show edit buffer instead of node content
            render_node_line_editing(tree_node, &app.edit_buffer)
        } else {
            render_and_track_node_line(tree_node, app, Rect {
                x: area.x + 1,
                y: area.y + 1 + (i - app.scroll_offset) as u16,
                width: area.width.saturating_sub(2),
                height: 1,
            })
        };
        
        // Highlight selected line
        if i == app.cursor_position {
            line = line.style(Style::default().bg(Color::Blue).fg(Color::White));
        }
        lines.push(line);

        // Phase 7: Render transclusions below the node (read-only)
        let re_trans = regex::Regex::new(r"!\[\[([^\]#]+)(?:#([^\]]+))?\]\]").unwrap();
        for cap in re_trans.captures_iter(&tree_node.node.content) {
            let title = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if title.is_empty() { continue; }
            let text_line = if let Ok(target) = NoteRepository::get_by_title_exact(&app.db_connection, title) {
                if let Some(node_id) = cap.get(2).map(|m| m.as_str().to_string()) {
                    if let Ok(tn) = NodeRepository::get_by_id(&app.db_connection, &node_id) {
                        format!("  ↳ {}", tn.content)
                    } else {
                        format!("  ↳ {} — (not found)", node_id)
                    }
                } else {
                    format!("  ↳ {}", target.title)
                }
            } else {
                format!("  ↳ {} — (missing note)", title)
            };
            let mut trans_line = Line::from(format!("{}{}", "  ".repeat(tree_node.depth + 1), text_line));
            trans_line = trans_line.style(Style::default().fg(Color::DarkGray));
            lines.push(trans_line);
        }

        // Limit to visible area
        if lines.len() >= (area.height as usize).saturating_sub(2) {
            break;
        }
    }

    let outline = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Outline ")
                .title_alignment(Alignment::Left),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(outline, area);

    if app.is_editing {
        if let Some(_node_id) = app.get_selected_node_id() {
            let visible_node = &app.get_visible_nodes()[app.cursor_position];
            let bullet_width = if visible_node.node.is_task { 2 } else if !visible_node.children.is_empty() { 2 } else { 2 };
            let indent_width = visible_node.depth as u16 * 2;
            let edit_area = Rect {
                x: area.x + 1 + indent_width + bullet_width,
                y: area.y + 1 + app.cursor_position as u16 - app.scroll_offset as u16,
                width: area.width.saturating_sub(2 + indent_width + bullet_width),
                height: 1,
            };

            let cursor_x = edit_area.x + app.edit_buffer[..app.edit_buffer.char_indices().map(|(i, _)| i).nth(app.edit_cursor_position).unwrap_or(app.edit_buffer.len())].width() as u16;

            frame.set_cursor(
                cursor_x,
                edit_area.y,
            );
        }
    }
}

/// Render a single node line and track link locations
fn render_and_track_node_line<'a>(tree_node: &'a TreeNode, app: &mut App, line_area: Rect) -> Line<'a> {
    let indent = "  ".repeat(tree_node.depth);
    let node = &tree_node.node;

    // Determine bullet point
    let bullet = if node.is_task {
        if node.task_completed { "☑ " } else { "☐ " }
    } else if !tree_node.children.is_empty() {
        if tree_node.is_expanded { "▼ " } else { "▶ " }
    } else {
        "• "
    };

    // Style based on node type
    let content_style = if node.is_task {
        if node.task_completed {
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::CROSSED_OUT)
        } else {
            Style::default().fg(Color::White)
        }
    } else if !tree_node.children.is_empty() {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
         match &node.block_type {
            notiq_core::models::BlockType::Quote => Style::default().fg(Color::Cyan).add_modifier(Modifier::ITALIC),
            notiq_core::models::BlockType::Code => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            notiq_core::models::BlockType::Normal => Style::default().fg(Color::White),
        }
    };

    // Priority indicator
    let priority_indicator = if node.is_task {
        match &node.task_priority {
            Some(p) => match p {
                notiq_core::models::TaskPriority::High => " 🔴",
                notiq_core::models::TaskPriority::Medium => " 🟡",
                notiq_core::models::TaskPriority::Low => " 🟢",
            },
            None => "",
        }
    } else {
        ""
    };

    let mut spans = vec![
        Span::raw(indent.clone()),
        Span::styled(bullet, Style::default().fg(Color::Cyan)),
    ];
    
    let mut current_x = line_area.x + indent.len() as u16 + bullet.len() as u16;

    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let mut last_index = 0;

    for cap in re.captures_iter(&node.content) {
        let full_match = cap.get(0).unwrap();
        let link_text = cap.get(1).unwrap();

        // Text before link
        let before_text = &node.content[last_index..full_match.start()];
        spans.push(Span::styled(before_text.to_string(), content_style));
        current_x += before_text.len() as u16;

        // The link
        let link_rect = Rect::new(current_x, line_area.y, full_match.as_str().len() as u16, 1);
        app.link_locations.push((link_rect, link_text.as_str().to_string()));

        spans.push(Span::styled(
            full_match.as_str().to_string(),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::UNDERLINED),
        ));
        current_x += full_match.as_str().len() as u16;
        last_index = full_match.end();
    }

    // Remaining text
    let after_text = &node.content[last_index..];
    spans.push(Span::styled(after_text.to_string(), content_style));
    spans.push(Span::raw(priority_indicator));
    
    Line::from(spans)
}


/// Render a node line when it's being edited (show edit buffer)
fn render_node_line_editing<'a>(tree_node: &TreeNode, edit_buffer: &'a str) -> Line<'a> {
    let indent = "  ".repeat(tree_node.depth);
    let node = &tree_node.node;

    // Determine bullet point
    let bullet = if node.is_task {
        if node.task_completed {
            "☑ "
        } else {
            "☐ "
        }
    } else if !tree_node.children.is_empty() {
        if tree_node.is_expanded {
            "▼ "
        } else {
            "▶ "
        }
    } else {
        "• "
    };

    let spans = vec![
        Span::raw(indent),
        Span::styled(bullet, Style::default().fg(Color::Cyan)),
        Span::styled(edit_buffer, Style::default().fg(Color::Yellow)),
        Span::styled("▊", Style::default().fg(Color::Yellow)), // Show cursor
    ];

    Line::from(spans)
}

/// Render the status bar at the bottom
pub fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let visible_count = app.get_visible_nodes().len();
    let status_text = if let Some(tag) = &app.tag_filter {
        format!(" {} nodes | Pages: {} | Tag Filter: #{} | [/:Search] [Ctrl+P: Switch] [Ctrl+N: New Page] [Ctrl+D: Delete Page] ", visible_count, app.notes.len(), tag)
    } else {
        format!(" {} nodes | Pages: {} | [/:Search] [Ctrl+P: Switch] [Ctrl+N: New Page] [Ctrl+D: Delete Page] ", visible_count, app.notes.len())
    };

    let status_bar = Paragraph::new(status_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .alignment(Alignment::Center);

    frame.render_widget(status_bar, area);
}

/// Render the sidebar pages list
pub fn render_sidebar_pages(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .notes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut line = Line::from(n.title.clone());
            if Some(&n.id) == app.current_note.as_ref().map(|cn| &cn.id) {
                line = line.style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
            }
            if i == app.sidebar_pages_selected_index {
                line = line.style(Style::default().bg(Color::Blue).fg(Color::Black));
            }
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    if !app.notes.is_empty() {
        state.select(Some(app.sidebar_pages_selected_index));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Pages ")
                .title_alignment(Alignment::Left),
        )
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Render sidebar with Tags panel (top) and Pages list (bottom)
pub fn render_sidebar_tags_and_pages(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Length(10), Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    // Calendar at the top
    render_calendar(frame, app, chunks[0]);

    // Tags panel (usage counts)
    let mut tag_lines: Vec<Line> = Vec::new();
    if let Ok(counts) = TagRepository::get_usage_counts(&app.db_connection) {
        for (tag, count) in counts.into_iter().take(8) {
            let mut line = Line::from(format!("#{} ({})", tag.name, count));
            if let Some(active) = &app.tag_filter { if *active == tag.name { line = line.style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)); } }
            tag_lines.push(line);
        }
    }
    if tag_lines.is_empty() { tag_lines.push(Line::from("No tags")); }
    let tags_widget = Paragraph::new(tag_lines)
        .block(Block::default().borders(Borders::ALL).title(" Tags "))
        .wrap(Wrap { trim: true });
    frame.render_widget(tags_widget, chunks[1]);

    // Favorites panel
    let mut fav_lines: Vec<Line> = Vec::new();
    if app.favorites.is_empty() {
        fav_lines.push(Line::from("No favorites"));
    } else {
        for fav in &app.favorites {
            let title = NoteRepository::get_by_id(&app.db_connection, &fav.note_id).map(|n| n.title).unwrap_or(fav.note_id.clone());
            fav_lines.push(Line::from(format!("⭐ {}", title)));
        }
    }
    let fav_widget = Paragraph::new(fav_lines)
        .block(Block::default().borders(Borders::ALL).title(" Favorites "))
        .wrap(Wrap { trim: true });
    frame.render_widget(fav_widget, chunks[2]);

    // Pages list below
    render_sidebar_pages(frame, app, chunks[3]);
}

/// Render backlinks panel for the current note
pub fn render_backlinks_panel(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    if let Some(current) = &app.current_note {
        if let Ok(links) = LinkRepository::get_backlinks(&app.db_connection, &current.id) {
            for link in links.into_iter().take((area.height as usize).saturating_sub(2)) {
                // Resolve source note title if possible
                let title = NoteRepository::get_by_id(&app.db_connection, &link.source_note_id)
                    .map(|n| n.title)
                    .unwrap_or(link.source_note_id);
                let text = if let Some(txt) = link.link_text { format!("{} — {}", title, txt) } else { title };
                lines.push(Line::from(text));
            }
        }
    }
    if lines.is_empty() { lines.push(Line::from("No backlinks")); }
    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Backlinks "))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

/// Render a simple logbook modal with entries for the selected task
pub fn render_logbook(frame: &mut Frame, app: &App, area: Rect) {
    if !app.logbook_open { return; }
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(40), Constraint::Percentage(30)])
        .split(area);
    let area_mid = popup_layout[1];
    let inner = Rect { x: area_mid.x + 1, y: area_mid.y + 1, width: area_mid.width.saturating_sub(2), height: area_mid.height.saturating_sub(2) };
    let block = Block::default().borders(Borders::ALL).title(" Log Book ");
    frame.render_widget(Clear, area_mid);
    frame.render_widget(block, area_mid);
    let mut lines: Vec<Line> = Vec::new();
    for log in &app.logbook_entries {
        let ts = log.timestamp.format("%Y-%m-%d %H:%M:%S");
        lines.push(Line::from(format!("{}: {} ({} -> {})", ts, log.status.to_string(), log.old_value.clone().unwrap_or_default(), log.new_value.clone().unwrap_or_default())));
    }
    if lines.is_empty() { lines.push(Line::from("No history")); }
    let para = Paragraph::new(lines).block(Block::default());
    frame.render_widget(para, inner);
}

/// Render attachments panel for the current note
pub fn render_attachments_panel(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::widgets::List;
    let mut items: Vec<ListItem> = Vec::new();
    for (i, att) in app.attachments.iter().enumerate() {
        let text = format!("{} ({}{}{})",
            att.filename,
            att.human_readable_size(),
            if let Some(mt) = &att.mime_type { ", ".to_string() + mt } else { String::new() },
            ""
        );
        let mut line = Line::from(text);
        if i == app.attachments_selected_index {
            line = line.style(Style::default().bg(Color::Blue).fg(Color::Black));
        }
        items.push(ListItem::new(line));
    }
    if items.is_empty() { items.push(ListItem::new(Line::from("No attachments"))); }
    let mut state = ListState::default();
    if !app.attachments.is_empty() {
        state.select(Some(app.attachments_selected_index));
    }
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Attachments "))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Render attach overlay to input a file path
pub fn render_attach_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(20), Constraint::Percentage(40)])
        .split(area);

    let area_mid = popup_layout[1];
    let inner_h = area_mid.height.saturating_sub(2);
    let inner_w = area_mid.width.saturating_sub(2);
    let inner_x = area_mid.x + 1;
    let inner_y = area_mid.y + 1;
    let inner = Rect { x: inner_x, y: inner_y, width: inner_w, height: inner_h };

    // Border and clear
    let block = Block::default().borders(Borders::ALL).title(" Attach File (Enter to confirm) ");
    frame.render_widget(Clear, area_mid);
    frame.render_widget(block, area_mid);

    let input = Paragraph::new(Text::from(format!("Path: {}", app.attach_input)))
        .style(Style::default().fg(Color::White))
        .block(Block::default());
    frame.render_widget(input, inner);
}

/// Render the search overlay with live results
pub fn render_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(50), Constraint::Percentage(25)])
        .split(area);

    let area_mid = popup_layout[1];
    let inner_h = area_mid.height.saturating_sub(2);
    let inner_w = area_mid.width.saturating_sub(2);
    let inner_x = area_mid.x + 1;
    let inner_y = area_mid.y + 1;
    let inner = Rect { x: inner_x, y: inner_y, width: inner_w, height: inner_h };

    // Border and clear
    let block = Block::default().borders(Borders::ALL).title(" Search ");
    frame.render_widget(Clear, area_mid);
    frame.render_widget(block, area_mid);

    // Split into input + results
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let input = Paragraph::new(Text::from(format!("/ {}", app.search_query)))
        .style(Style::default().fg(Color::White))
        .block(Block::default());
    frame.render_widget(input, inner_chunks[0]);

    // Results list
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .map(|n| ListItem::new(Line::from(n.content.clone())))
        .collect();
    let list = List::new(items).block(Block::default());
    frame.render_widget(list, inner_chunks[1]);
}

/// Render the page switcher overlay (center modal with filter input and list)
pub fn render_page_switcher(frame: &mut Frame, app: &App, area: Rect) {
    // Centered box
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(35),
        ])
        .split(area);

    let area_mid = popup_layout[1];
    let inner_h = area_mid.height.saturating_sub(2);
    let inner_w = area_mid.width.saturating_sub(2);
    let inner_x = area_mid.x + 1;
    let inner_y = area_mid.y + 1;
    let inner = Rect { x: inner_x, y: inner_y, width: inner_w, height: inner_h };

    // Draw border and clear background
    let block = Block::default().borders(Borders::ALL).title(" Page Switcher ");
    frame.render_widget(Clear, area_mid);
    frame.render_widget(block, area_mid);

    // Split inner into filter input + list
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    // Filter line
    let filter = Paragraph::new(Text::from(format!("> {}", app.page_filter)))
        .style(Style::default().fg(Color::White))
        .block(Block::default());
    frame.render_widget(filter, inner_chunks[0]);

    // List of filtered notes
    let filtered = app.get_filtered_notes();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut line = Line::from(n.title.clone());
            if i == app.page_switcher_selection_index {
                line = line.style(Style::default().bg(Color::Blue).fg(Color::Black));
            }
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    if !filtered.is_empty() {
        state.select(Some(app.page_switcher_selection_index));
    }

    let list = List::new(items)
        .block(Block::default())
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black));
    frame.render_stateful_widget(list, inner_chunks[1], &mut state);
}

/// Render a simple month calendar with current day and selection highlights
pub fn render_calendar(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let month_start = app.calendar_month_start;
    let title = format!("{} {}", month_start.format("%B"), month_start.year());
    lines.push(Line::from(Span::styled(title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
    lines.push(Line::from(" Mo Tu We Th Fr Sa Su"));

    // Determine grid start (Monday as first column)
    // Calculate which weekday the 1st of the month falls on
    let first_day_of_month = NaiveDate::from_ymd_opt(month_start.year(), month_start.month(), 1).unwrap();
    let first_weekday = match first_day_of_month.weekday() { 
        Weekday::Mon => 0, Weekday::Tue => 1, Weekday::Wed => 2, Weekday::Thu => 3, 
        Weekday::Fri => 4, Weekday::Sat => 5, Weekday::Sun => 6 
    };
    let mut day = 1i32;
    let days_in_month = days_in_month(month_start.year(), month_start.month());
    let today = chrono::Utc::now().date_naive();

    // Up to 6 rows
    for row in 0..6 {
        let mut row_spans: Vec<Span> = Vec::new();
        for col in 0..7 {
            let mut text = "   ".to_string(); // 3 spaces for alignment
            let cell_index = row * 7 + col;
            
            // Check if this cell should contain a day number
            if cell_index >= first_weekday && day <= days_in_month as i32 {
                text = format!(" {:<2}", day); // Pad to 3 chars
                let date = NaiveDate::from_ymd_opt(month_start.year(), month_start.month(), day as u32)
                    .unwrap_or(month_start);
                let mut style = Style::default().fg(Color::White);
                if date == today {
                    style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                }
                if date == app.calendar_selected {
                    style = style.bg(Color::Blue).fg(Color::Black);
                }
                row_spans.push(Span::styled(text, style));
                day += 1;
            } else {
                row_spans.push(Span::raw(text));
            }
            
            // Add spacing between columns (except after the last column)
            if col < 6 { 
                row_spans.push(Span::raw(" ")); 
            }
        }
        lines.push(Line::from(row_spans));
        if day > days_in_month as i32 { break; }
    }

    let widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Calendar "))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

fn days_in_month(year: i32, month: u32) -> u32 {
    // Next month first day minus one day
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    let last_this = first_next - chrono::Duration::days(1);
    last_this.day()
}

pub fn render_delete_confirmation(frame: &mut Frame, _app: &App, area: Rect) {
    let popup_width = 60;
    let popup_height = 5;

    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let text = "Are you sure you want to delete this node and all its children? (y/n)";
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title("Confirm Deletion")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);

    frame.render_widget(Clear, popup_area); // This clears the area behind the popup
    frame.render_widget(paragraph, popup_area);
}

/// Render autocomplete popup
pub fn render_autocomplete(frame: &mut Frame, app: &App, _area: Rect) {
    if !app.autocomplete_open || app.autocomplete_items.is_empty() {
        return;
    }

    // Small popup near the cursor
    let popup_width = 40;
    let popup_height = 10.min(app.autocomplete_items.len() as u16 + 2);

    let x = 10; // Simplified positioning
    let y = 5;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let title = match app.autocomplete_type {
        crate::app::AutocompleteType::WikiLink => " Link Suggestions [[  ",
        crate::app::AutocompleteType::Tag => " Tag Suggestions #  ",
        crate::app::AutocompleteType::None => " Suggestions ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(block.clone(), popup_area);

    // Inner content area
    let inner = Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(2),
    };

    // Render items
    let items: Vec<ListItem> = app.autocomplete_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut line = Line::from(item.clone());
            if i == app.autocomplete_selection {
                line = line.style(Style::default().bg(Color::Blue).fg(Color::White));
            }
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.autocomplete_selection));

    let list = List::new(items)
        .block(Block::default())
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

    frame.render_stateful_widget(list, inner, &mut state);
}

/// Render task overview panel
pub fn render_task_overview(frame: &mut Frame, app: &App, area: Rect) {
    if !app.task_overview_open {
        return;
    }

    // Large centered popup
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(area);

    let popup_area = popup_layout[1];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Task Overview (x/Space:Toggle | Enter:Go To | Esc:Close) ")
        .style(Style::default().fg(Color::Yellow));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(block.clone(), popup_area);

    // Inner content
    let inner = Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(2),
    };

    if app.task_overview_tasks.is_empty() {
        let para = Paragraph::new("No tasks found")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(para, inner);
        return;
    }

    // Render task list
    let items: Vec<ListItem> = app.task_overview_tasks
        .iter()
        .enumerate()
        .map(|(i, task_item)| {
            let checkbox = if task_item.node.task_completed { "☑" } else { "☐" };
            let priority_icon = match &task_item.node.task_priority {
                Some(notiq_core::models::TaskPriority::High) => "🔴",
                Some(notiq_core::models::TaskPriority::Medium) => "🟡",
                Some(notiq_core::models::TaskPriority::Low) => "🟢",
                None => "  ",
            };
            
            let text = format!(
                "{} {} {} — {}",
                checkbox,
                priority_icon,
                task_item.node.content,
                task_item.note_title
            );

            let mut line = Line::from(text);
            if i == app.task_overview_selection {
                line = line.style(Style::default().bg(Color::Blue).fg(Color::White));
            } else if task_item.node.task_completed {
                line = line.style(Style::default().fg(Color::DarkGray));
            }

            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.task_overview_selection));

    let list = List::new(items)
        .block(Block::default())
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

    frame.render_stateful_widget(list, inner, &mut state);
}


/// Render overlay for renaming the current page
pub fn render_rename_page_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let popup_width = 80;
    let popup_height = 5;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Rename Page (Enter:Save | Esc:Cancel) ")
        .style(Style::default().fg(Color::Cyan));
    
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 2,
        width: popup_area.width.saturating_sub(2),
        height: 1,
    };
    
    let text = format!("{}▊", app.page_title_buffer);
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::Yellow));
        
    frame.render_widget(paragraph, inner);
}

/// Render the help screen overlay
pub fn render_help_screen(frame: &mut Frame, _app: &App, size: Rect) {
    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("Navigation", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("↑/↓          Move cursor up/down"),
        Line::from("←/→          Expand/collapse nodes"),
        Line::from("Tab          Indent node"),
        Line::from("Shift+Tab    Outdent node"),
        Line::from("Alt+↑/↓      Reorder nodes"),
        Line::from(""),
        Line::from(Span::styled("Editing", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("Enter        Edit node"),
        Line::from("Esc          Cancel edit"),
        Line::from("n            Create new node"),
        Line::from("Insert       Create new node"),
        Line::from("d            Delete node"),
        Line::from("Delete       Delete node"),
        Line::from("x            Toggle task completion"),
        Line::from("Ctrl+Q       Create quote block"),
        Line::from("Ctrl+C       Create code block"),
        Line::from(""),
        Line::from(Span::styled("Pages", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("Ctrl+P       Page switcher"),
        Line::from("Ctrl+N       New page"),
        Line::from("Ctrl+D       Delete page"),
        Line::from("Ctrl+R       Rename page"),
        Line::from("Ctrl+F       Toggle favorite"),
        Line::from(""),
        Line::from(Span::styled("Search & Links", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("/            Search"),
        Line::from("#tag         Filter by tag"),
        Line::from("[[Page]]     Create link"),
        Line::from("![[Page]]    Transclude content"),
        Line::from(""),
        Line::from(Span::styled("Calendar & Tasks", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("Shift+Arrow  Navigate calendar"),
        Line::from("Shift+Enter  Open daily note"),
        Line::from("Ctrl+Shift+T Task overview"),
        Line::from("Ctrl+L       Open logbook"),
        Line::from(""),
        Line::from(Span::styled("Files & Export", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("Ctrl+A       Attach file"),
        Line::from("Ctrl+V       Paste image"),
        Line::from("Ctrl+O       Open attachments"),
        Line::from("Ctrl+E       Export to Markdown"),
        Line::from("[[/]]        Navigate attachments"),
        Line::from(""),
        Line::from(Span::styled("Interface", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("Ctrl+B       Toggle sidebar"),
        Line::from("h            Show this help"),
        Line::from("q            Quit application"),
        Line::from(""),
        Line::from(Span::styled("Special Characters", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("AltGr+[      Square brackets"),
        Line::from("AltGr+]      Square brackets"),
        Line::from("AltGr+{      Curly braces"),
        Line::from("AltGr+}      Curly braces"),
        Line::from("AltGr+@      At symbol"),
        Line::from("AltGr+#      Hash symbol"),
        Line::from(""),
        Line::from(Span::styled("Press 'h' or 'Esc' to close", Style::default().fg(Color::DarkGray))),
    ];

    let popup_width = 80;
    let popup_height = (help_text.len() as u16 + 2).min(size.height);
    let x = (size.width.saturating_sub(popup_width)) / 2;
    let y = (size.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);
    
    let block = Block::default()
        .title(" Help - Keyboard Shortcuts ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x + 1,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(2),
        height: popup_area.height.saturating_sub(2),
    };

    let paragraph = Paragraph::new(help_text)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
        
    frame.render_widget(paragraph, inner);
}

