use anyhow::Result;
use notiq_core::{
    models::{Attachment, Note, OutlineNode, TaskStatus, TaskStatusLog},
    storage::{
        AttachmentRepository, Connection, DailyNoteRepository, Database, FavoriteRepository, LinkRepository,
        NodeRepository, NoteRepository, TagRepository, TaskLogRepository,
    },
};
use chrono::{Datelike, Duration, NaiveDate};
use std::io::Read;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use std::time::Instant;
use ratatui::layout::Rect;
use crate::config::{Config, load_config};
use std::collections::HashMap;

/// Represents a node in the outline tree with its children
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub node: OutlineNode,
    pub children: Vec<TreeNode>,
    pub is_expanded: bool,
    pub depth: usize,
}

impl TreeNode {
    pub fn new(node: OutlineNode, depth: usize) -> Self {
        Self {
            node,
            children: Vec::new(),
            is_expanded: true,
            depth,
        }
    }

    /// Build a tree structure from a flat list of nodes
    pub fn build_tree(nodes: Vec<OutlineNode>) -> Vec<TreeNode> {
        let mut root_nodes = Vec::new();
        let mut node_map: std::collections::HashMap<String, Vec<OutlineNode>> = std::collections::HashMap::new();

        // Group nodes by parent
        for node in nodes {
            if let Some(parent_id) = &node.parent_node_id {
                node_map.entry(parent_id.clone()).or_default().push(node);
            } else {
                root_nodes.push(node);
            }
        }

        // Recursively build tree
        fn build_subtree(
            node: OutlineNode,
            node_map: &std::collections::HashMap<String, Vec<OutlineNode>>,
            depth: usize,
        ) -> TreeNode {
            let mut tree_node = TreeNode::new(node.clone(), depth);
            
            if let Some(children) = node_map.get(&node.id) {
                tree_node.children = children
                    .iter()
                    .cloned()
                    .map(|child| build_subtree(child, node_map, depth + 1))
                    .collect();
            }
            
            tree_node
        }

        root_nodes
            .into_iter()
            .map(|node| build_subtree(node, &node_map, 0))
            .collect()
    }

    /// Flatten the tree for display, respecting expanded/collapsed state
    pub fn flatten(&self) -> Vec<&TreeNode> {
        let mut result = vec![self];
        
        if self.is_expanded {
            for child in &self.children {
                result.extend(child.flatten());
            }
        }
        
        result
    }
}

/// Application state
pub struct App {
    pub should_quit: bool,
    pub current_note: Option<Note>,
    pub outline_tree: Vec<TreeNode>,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub db_connection: Connection,
    pub config: Config,
    pub is_editing: bool,
    pub edit_buffer: String,
    pub edit_cursor_position: usize,
    // Phase 4 - Pages management
    pub notes: Vec<Note>,
    pub sidebar_pages_selected_index: usize,
    pub page_switcher_open: bool,
    pub page_filter: String,
    pub page_switcher_selection_index: usize,
    // Phase 5 - Search & Tags & Backlinks
    pub search_open: bool,
    pub search_query: String,
    pub search_results: Vec<OutlineNode>,
    pub tag_filter: Option<String>,
    // Phase 6 - Calendar & Daily Notes
    pub calendar_month_start: NaiveDate,
    pub calendar_selected: NaiveDate,
    // Phase 7 - Attachments
    pub attachments: Vec<Attachment>,
    pub attachments_selected_index: usize,
    pub attach_overlay_open: bool,
    pub attach_input: String,
    pub workspace_dir: PathBuf,
    // Favorites
    pub favorites: Vec<notiq_core::models::Favorite>,
    pub favorites_selected_index: usize,
    pub logbook_open: bool,
    pub logbook_entries: Vec<notiq_core::models::TaskStatusLog>,
    pub show_sidebar: bool,
    pub last_input_time: Option<Instant>,
    pub confirming_delete: bool,
    pub pending_delete_node_id: Option<String>,
    // Autocomplete state
    pub autocomplete_open: bool,
    pub autocomplete_type: AutocompleteType,
    pub autocomplete_items: Vec<String>,
    pub autocomplete_selection: usize,
    pub autocomplete_trigger_pos: usize,
    // Task overview
    pub task_overview_open: bool,
    pub task_overview_tasks: Vec<TaskOverviewItem>,
    pub task_overview_selection: usize,
    // Page renaming
    pub is_renaming_page: bool,
    pub page_title_buffer: String,
    // Help screen
    pub help_open: bool,
    // Clickable links tracking
    pub link_locations: Vec<(Rect, String)>,
    // Search state
    pub search_open: bool,
    pub search_query: String,
    pub search_results: Vec<OutlineNode>,
    pub search_selection: usize,
    pub current_note_nodes: Vec<OutlineNode>,
    pub current_note_attachments: HashMap<String, Vec<Attachment>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AutocompleteType {
    None,
    WikiLink,  // [[
    Tag,       // #
}

#[derive(Debug, Clone)]
pub struct TaskOverviewItem {
    pub node: OutlineNode,
    pub note_title: String,
    pub note_id: String,
}

impl App {
    /// Create a new App instance
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Database::new(db_path);
        let conn = db.get_or_create()?;
        let config_path = PathBuf::from(db_path)
            .parent()
            .map(|p| p.join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("."));
        let config = load_config(&config_path);
        let today = chrono::Utc::now().date_naive();
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .unwrap_or(today);
        let db_pathbuf = PathBuf::from(db_path);
        let workspace_dir = db_pathbuf
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        
        Ok(Self {
            should_quit: false,
            current_note: None,
            outline_tree: Vec::new(),
            cursor_position: 0,
            scroll_offset: 0,
            db_connection: conn,
            config,
            is_editing: false,
            edit_buffer: String::new(),
            edit_cursor_position: 0,
            notes: Vec::new(),
            sidebar_pages_selected_index: 0,
            page_switcher_open: false,
            page_filter: String::new(),
            page_switcher_selection_index: 0,
            search_open: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selection: 0,
            tag_filter: None,
            calendar_month_start: month_start,
            calendar_selected: today,
            attachments: Vec::new(),
            attachments_selected_index: 0,
            attach_overlay_open: false,
            attach_input: String::new(),
            workspace_dir,
            favorites: Vec::new(),
            favorites_selected_index: 0,
            logbook_open: false,
            logbook_entries: Vec::new(),
            show_sidebar: true,
            last_input_time: None,
            confirming_delete: false,
            pending_delete_node_id: None,
            autocomplete_open: false,
            autocomplete_type: AutocompleteType::None,
            autocomplete_items: Vec::new(),
            autocomplete_selection: 0,
            autocomplete_trigger_pos: 0,
            task_overview_open: false,
            task_overview_tasks: Vec::new(),
            task_overview_selection: 0,
            // Page renaming
            is_renaming_page: false,
            page_title_buffer: String::new(),
            // Help screen
            help_open: false,
            // Clickable links
            link_locations: Vec::new(),
            current_note_nodes: Vec::new(),
            current_note_attachments: HashMap::new(),
        })
    }

    /// Initialize with sample data if database is empty
    pub fn initialize_sample_data(&mut self) -> Result<()> {
        let note_count = NoteRepository::count(&self.db_connection)?;
        
        if note_count == 0 {
            // Create a sample note
            let note = Note::new("Welcome to Notiq".to_string());
            NoteRepository::create(&self.db_connection, &note)?;
            
            // Create sample outline
            let nodes = vec![
                OutlineNode::new(note.id.clone(), None, "🎉 Welcome! This is a simple outliner application.".to_string(), 0),
                OutlineNode::new(note.id.clone(), None, "Features".to_string(), 1),
                OutlineNode::new_task(note.id.clone(), None, "Infinite nesting support".to_string(), 2, None, None),
                OutlineNode::new_task(note.id.clone(), None, "Task management with checkboxes".to_string(), 3, None, None),
                OutlineNode::new(note.id.clone(), None, "Getting Started".to_string(), 4),
                OutlineNode::new(note.id.clone(), None, "Navigation".to_string(), 5),
            ];
            
            // Create nodes and build hierarchy
            let _root1_id = nodes[0].id.clone();
            let root2_id = nodes[1].id.clone();
            let root5_id = nodes[4].id.clone();
            let root6_id = nodes[5].id.clone();
            
            for node in &nodes[0..6] {
                NodeRepository::create(&self.db_connection, node)?;
            }
            
            // Add children to "Features"
            let feature_child1 = OutlineNode::new(
                note.id.clone(),
                Some(root2_id.clone()),
                "SQLite database backend".to_string(),
                0,
            );
            let feature_child2 = OutlineNode::new(
                note.id.clone(),
                Some(root2_id.clone()),
                "Full-text search".to_string(),
                1,
            );
            NodeRepository::create(&self.db_connection, &feature_child1)?;
            NodeRepository::create(&self.db_connection, &feature_child2)?;
            
            // Add children to "Getting Started"
            let getting_started_child = OutlineNode::new(
                note.id.clone(),
                Some(root5_id.clone()),
                "Press 'q' to quit the application".to_string(),
                0,
            );
            NodeRepository::create(&self.db_connection, &getting_started_child)?;
            
            // Add children to "Navigation"
            let nav_children = vec![
                OutlineNode::new(note.id.clone(), Some(root6_id.clone()), "↑/↓ - Navigate up and down (Phase 3)".to_string(), 0),
                OutlineNode::new(note.id.clone(), Some(root6_id.clone()), "←/→ - Collapse/Expand nodes (Phase 3)".to_string(), 1),
                OutlineNode::new(note.id.clone(), Some(root6_id.clone()), "Enter - Edit node (Phase 3)".to_string(), 2),
            ];
            
            for child in nav_children {
                NodeRepository::create(&self.db_connection, &child)?;
            }
        }
        
        Ok(())
    }

    /// Load a note and its outline
    pub fn load_note(&mut self, note_id: &str) -> Result<()> {
        let note = NoteRepository::get_by_id(&self.db_connection, note_id)?;
        let nodes = NodeRepository::get_by_note_id(&self.db_connection, note_id)?;
        
        self.current_note = Some(note);
        self.outline_tree = TreeNode::build_tree(nodes);
        self.cursor_position = 0;
        self.scroll_offset = 0;
        self.refresh_attachments()?;
        
        // Also load attachments for this note
        let attachments = AttachmentRepository::get_by_note_id(&self.db_connection, note_id)?;
        let mut map = HashMap::new();
        for att in attachments {
            map.entry(att.node_id.clone()).or_default().push(att);
        }
        self.current_note_attachments = map;

        self.refresh_backlinks()
    }

    /// Load the first available note
    pub fn load_first_note(&mut self) -> Result<()> {
        self.refresh_notes_list()?;
        if let Some(note) = self.notes.first() {
            let id = note.id.clone();
            self.load_note(&id)?;
            self.sidebar_pages_selected_index = 0;
        }

        Ok(())
    }

    /// Get all visible nodes (flattened tree)
    pub fn get_visible_nodes(&self) -> Vec<&TreeNode> {
        self.outline_tree
            .iter()
            .flat_map(|node| node.flatten())
            .collect()
    }

    /// Build a list of visible paths (indices into the tree). Each path represents a visible node.
    fn build_visible_paths(&self) -> Vec<Vec<usize>> {
        fn walk(node: &TreeNode, path: &mut Vec<usize>, acc: &mut Vec<Vec<usize>>) {
            acc.push(path.clone());
            if node.is_expanded {
                for (i, child) in node.children.iter().enumerate() {
                    path.push(i);
                    walk(child, path, acc);
                    path.pop();
                }
            }
        }

        let mut paths = Vec::new();
        for (i, node) in self.outline_tree.iter().enumerate() {
            let mut path = vec![i];
            walk(node, &mut path, &mut paths);
        }
        paths
    }

    /// Get mutable reference to a tree node by its path
    fn get_node_mut_by_path(&mut self, path: &[usize]) -> Option<&mut TreeNode> {
        if path.is_empty() { return None; }
        let mut current: *mut TreeNode = match self.outline_tree.get_mut(path[0]) {
            Some(n) => n,
            None => return None,
        } as *mut TreeNode;

        // Safety: We only use one mutable borrow chain at a time
        for idx in &path[1..] {
            unsafe {
                let cur_ref: &mut TreeNode = &mut *current;
                current = match cur_ref.children.get_mut(*idx) {
                    Some(n) => n,
                    None => return None,
                } as *mut TreeNode;
            }
        }

        unsafe { Some(&mut *current) }
    }

    /// Get the selected node's ID (if any)
    pub fn get_selected_node_id(&self) -> Option<String> {
        let visible = self.get_visible_nodes();
        visible.get(self.cursor_position).map(|t| t.node.id.clone())
    }

    /// Toggle expansion state of the selected node
    pub fn toggle_selected_expand_collapse(&mut self, expand: Option<bool>) {
        let paths = self.build_visible_paths();
        if let Some(path) = paths.get(self.cursor_position) {
            if let Some(node) = self.get_node_mut_by_path(path) {
                if !node.children.is_empty() {
                    match expand {
                        Some(true) => node.is_expanded = true,
                        Some(false) => node.is_expanded = false,
                        None => node.is_expanded = !node.is_expanded,
                    }
                }
            }
        }
    }

    /// Move cursor up (saturating at 0)
    pub fn move_cursor_up(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            if self.cursor_position < self.scroll_offset {
                self.scroll_offset = self.cursor_position;
            }
        }
    }

    /// Move cursor down (saturating at last visible)
    pub fn move_cursor_down(&mut self) {
        let last = self.get_visible_nodes().len().saturating_sub(1);
        if self.cursor_position < last {
            self.cursor_position += 1;
        }
    }

    /// Start editing the selected node
    pub fn start_editing(&mut self) {
        if self.is_editing { return; }
        if let Some(id) = self.get_selected_node_id() {
            if let Ok(node) = NodeRepository::get_by_id(&self.db_connection, &id) {
                self.edit_buffer = node.content.clone();
                self.edit_cursor_position = self.edit_buffer.chars().count();
                self.is_editing = true;
            }
        }
    }

    /// Cancel edit mode without saving
    pub fn cancel_edit(&mut self) {
        self.is_editing = false;
        self.edit_buffer.clear();
        self.edit_cursor_position = 0;
    }

    /// Commit edit buffer to the database and refresh
    pub fn commit_edit(&mut self) -> Result<()> {
        if !self.is_editing { return Ok(()); }
        let selected_id = match self.get_selected_node_id() { Some(id) => id, None => return Ok(()) };
        let mut node = NodeRepository::get_by_id(&self.db_connection, &selected_id)?;
        node.content = self.edit_buffer.clone();
        // Phase 6: parse task checkbox markers in content
        Self::apply_task_parsing(&mut node);
        node.touch();
        NodeRepository::update(&self.db_connection, &node)?;
        // Phase 5: update tags and links after content change
        self.update_tags_and_links_for_node(&node)?;
        self.is_editing = false;
        self.edit_buffer.clear();
        self.edit_cursor_position = 0;
        self.refresh_current_note_preserve_selection(Some(&selected_id))?;
        Ok(())
    }

    /// Phase 6: Detect [ ] / [x] prefix to set task flags on the node
    fn apply_task_parsing(node: &mut OutlineNode) {
        let trimmed = node.content.trim_start();
        // Accept variants: "[ ] ", "[x] ", "[X] " at the start (after leading spaces)
        let checkbox_unchecked = trimmed.starts_with("[ ] ") || trimmed.starts_with("[ ]\t");
        let checkbox_checked = trimmed.starts_with("[x] ") || trimmed.starts_with("[X] ")
            || trimmed.starts_with("[x]\t") || trimmed.starts_with("[X]\t");

        if checkbox_unchecked || checkbox_checked {
            node.is_task = true;
            node.task_completed = checkbox_checked;
            // Strip the checkbox from stored content for cleaner text rendering
            let without = if checkbox_unchecked { &trimmed[4..] } else { &trimmed[4..] };
            // Preserve original leading spaces count
            let leading_ws_len = node.content.len() - trimmed.len();
            let leading_ws = &node.content[..leading_ws_len];
            node.content = format!("{}{}", leading_ws, without);
        } else {
            // If no checkbox marker, do not force reset is_task; user may have task without marker
            // However, if content was emptied of marker and node had been auto-task before, keep as-is
        }
    }

    // =========================
    // Phase 6: Task toggle + log
    // =========================
    pub fn toggle_selected_task(&mut self) -> Result<()> {
        let selected_id = match self.get_selected_node_id() { Some(id) => id, None => return Ok(()) };
        let mut node = NodeRepository::get_by_id(&self.db_connection, &selected_id)?;
        if !node.is_task { return Ok(()); }
        let old = node.task_completed;
        let now_completed = node.toggle_task();
        NodeRepository::update(&self.db_connection, &node)?;

        // Log status change
        let status = if now_completed { TaskStatus::Completed } else { TaskStatus::Uncompleted };
        let log = TaskStatusLog::new(
            node.id.clone(),
            status,
            Some(old.to_string()),
            Some(now_completed.to_string()),
        );
        let _ = TaskLogRepository::create(&self.db_connection, &log)?;

        self.refresh_current_note_preserve_selection(Some(&selected_id))?;
        Ok(())
    }

    // =========================
    // Phase 6: Calendar helpers
    // =========================
    pub fn calendar_move_day(&mut self, delta: i64) {
        self.calendar_selected = self.calendar_selected + Duration::days(delta);
        // Keep month view aligned with selected date's month
        self.calendar_month_start = NaiveDate::from_ymd_opt(
            self.calendar_selected.year(),
            self.calendar_selected.month(),
            1,
        ).unwrap_or(self.calendar_selected);
    }

    pub fn calendar_move_week(&mut self, delta_weeks: i64) {
        self.calendar_move_day(delta_weeks * 7);
    }

    pub fn calendar_prev_month(&mut self) {
        let y = self.calendar_month_start.year();
        let m = self.calendar_month_start.month();
        let (ny, nm) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
        self.calendar_month_start = NaiveDate::from_ymd_opt(ny, nm, 1)
            .unwrap_or(self.calendar_month_start);
        // Clamp selected to same day if in same month else first day
        if self.calendar_selected.year() != ny || self.calendar_selected.month() != nm {
            self.calendar_selected = self.calendar_month_start;
        }
    }

    pub fn calendar_next_month(&mut self) {
        let y = self.calendar_month_start.year();
        let m = self.calendar_month_start.month();
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        self.calendar_month_start = NaiveDate::from_ymd_opt(ny, nm, 1)
            .unwrap_or(self.calendar_month_start);
        if self.calendar_selected.year() != ny || self.calendar_selected.month() != nm {
            self.calendar_selected = self.calendar_month_start;
        }
    }

    pub fn calendar_goto_today(&mut self) {
        let today = chrono::Utc::now().date_naive();
        self.calendar_selected = today;
        self.calendar_month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
            .unwrap_or(today);
    }

    /// Open or create the daily note for the selected date
    pub fn open_selected_daily_note(&mut self) -> Result<()> {
        let date = self.calendar_selected;
        // Try existing daily note
        match DailyNoteRepository::get_by_date(&self.db_connection, date) {
            Ok(daily) => {
                self.load_note(&daily.note_id)?;
            }
            Err(_) => {
                // Create a new note and associate
                let title = format!("{} Daily Note", date.format("%Y-%m-%d"));
                let note = Note::new(title);
                NoteRepository::create(&self.db_connection, &note)?;
                let _ = DailyNoteRepository::get_or_create(
                    &self.db_connection,
                    date,
                    note.id.clone(),
                )?;
                self.load_note(&note.id)?;
                self.refresh_notes_list()?; // include in pages list
            }
        }
        Ok(())
    }

    /// Phase 5: Parse tags and wiki links, persist associations
    fn update_tags_and_links_for_node(&mut self, node: &OutlineNode) -> Result<()> {
        // Parse tags like #tag-name
        let re_tags = regex::Regex::new(r"(?P<tag>#([A-Za-z0-9_-]+))").unwrap();
        let mut tags: Vec<String> = re_tags
            .captures_iter(&node.content)
            .filter_map(|c| c.get(2).map(|m| m.as_str().to_string()))
            .collect();
        tags.sort();
        tags.dedup();
        TagRepository::set_tags_for_node(&self.db_connection, &node.id, &tags)?;

        // Refresh links: delete old ones for this node, then create from [[Title]] and transclusions
        LinkRepository::delete_by_source_node(&self.db_connection, &node.id)?;
        let re_links = regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
        for cap in re_links.captures_iter(&node.content) {
            // Skip if it's a transclusion (preceded by '!')
            if let Some(m) = cap.get(0) {
                let s = m.start();
                if s > 0 && node.content.as_bytes()[s - 1] == b'!' { continue; }
            }
            let title = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if title.is_empty() { continue; }

            let target_note = NoteRepository::get_by_title_exact(&self.db_connection, title);
            let source_note_id = match &self.current_note { Some(n) => n.id.clone(), None => continue };

            match target_note {
                Ok(target) => {
                    let link = notiq_core::models::Link::new_wiki_link(
                        source_note_id,
                        Some(node.id.clone()),
                        target.id,
                        Some(title.to_string()),
                    );
                    let _ = LinkRepository::create(&self.db_connection, &link)?;
                },
                Err(notiq_core::Error::NotFound(_)) => {
                    // Auto-create page
                    let new_note = notiq_core::models::Note::new(title.to_string());
                    NoteRepository::create(&self.db_connection, &new_note)?;

                    // Forward link
                    let link = notiq_core::models::Link::new_wiki_link(
                        source_note_id,
                        Some(node.id.clone()),
                        new_note.id.clone(),
                        Some(title.to_string()),
                    );
                    let _ = LinkRepository::create(&self.db_connection, &link)?;

                    // Backlink
                    if let Some(source_note) = &self.current_note {
                        let backlink_content = format!("[[{}]]", source_note.title);
                        let backlink_node = notiq_core::models::OutlineNode::new(new_note.id.clone(), None, backlink_content);
                        NodeRepository::create(&self.db_connection, &backlink_node)?;
                    }
                },
                Err(_) => { /* Other DB errors, do nothing */ }
            }
        }

        // Transclusions: ![[Note Title#OptionalNodeIdOrHeader]]
        let re_trans = regex::Regex::new(r"!\[\[([^\]#]+)(?:#([^\]]+))?\]\]").unwrap();
        for cap in re_trans.captures_iter(&node.content) {
            let title = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if title.is_empty() { continue; }
            if let Ok(target) = NoteRepository::get_by_title_exact(&self.db_connection, title) {
                let source_note_id = match &self.current_note { Some(n) => n.id.clone(), None => continue };
                let text = cap.get(2).map(|m| m.as_str().to_string());
                let link = notiq_core::models::Link::new_transclusion(
                    source_note_id,
                    Some(node.id.clone()),
                    target.id,
                    text,
                );
                let _ = LinkRepository::create(&self.db_connection, &link)?;
            }
        }
        Ok(())
    }

    /// Create a new sibling node below the current selection
    pub fn create_sibling_below(&mut self) -> Result<()> {
        let note_id = match &self.current_note { Some(n) => n.id.clone(), None => return Ok(()) };
        let selected_paths = self.build_visible_paths();

        if selected_paths.is_empty() {
            // No nodes on page, create a new root node.
            let next_pos = NodeRepository::get_next_child_position(&self.db_connection, None, &note_id)?;
            let new_node = OutlineNode::new(note_id, None, "".to_string(), next_pos);
            let new_id = new_node.id.clone();
            NodeRepository::create(&self.db_connection, &new_node)?;
            self.refresh_current_note_preserve_selection(Some(&new_id))?;
            self.start_editing();
        } else if let Some(path) = selected_paths.get(self.cursor_position) {
            // Determine parent of selected
            let parent_id_opt = if path.len() == 1 {
                None
            } else {
                // Get parent path
                let parent_path = &path[..path.len()-1];
                let parent_node = self.get_node_by_path_readonly(parent_path);
                parent_node.map(|n| n.node.id.clone())
            };

            // Next position among siblings
            let next_pos = NodeRepository::get_next_child_position(
                &self.db_connection,
                parent_id_opt.as_deref(),
                &note_id,
            )?;

            let new_node = OutlineNode::new(note_id.clone(), parent_id_opt.clone(), "".to_string(), next_pos);
            let new_id = new_node.id.clone();
            NodeRepository::create(&self.db_connection, &new_node)?;
            self.refresh_current_note_preserve_selection(Some(&new_id))?;

            // Start editing the new node immediately
            self.start_editing();
        }
        Ok(())
    }

    /// Delete the selected node
    pub fn initiate_delete(&mut self) {
        if let Some(id) = self.get_selected_node_id() {
            self.pending_delete_node_id = Some(id);
            self.confirming_delete = true;
        }
    }

    pub fn confirm_delete(&mut self) -> Result<()> {
        if let Some(id) = self.pending_delete_node_id.take() {
            NodeRepository::delete(&self.db_connection, &id)?;
            // Move cursor up if needed
            if self.cursor_position > 0 { self.cursor_position -= 1; }
            self.refresh_current_note_preserve_selection(None)?;
        }
        self.confirming_delete = false;
        Ok(())
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete_node_id = None;
        self.confirming_delete = false;
    }

    /// Indent the selected node (make it a child of previous visible sibling)
    pub fn indent_selected(&mut self) -> Result<()> {
        let paths = self.build_visible_paths();
        if let Some(path) = paths.get(self.cursor_position) {
            if path.is_empty() { return Ok(()); }
            // Need previous visible node that is a sibling or ancestor sibling
            if path.last() == Some(&0) { return Ok(()); } // first child cannot indent relative to prev sibling
            // Previous sibling within same parent
            let parent_path = path[..path.len()-1].to_vec();
            let idx_in_parent = *path.last().unwrap();
            if idx_in_parent == 0 { return Ok(()); }
            let prev_sibling_path = {
                let mut p = parent_path.clone();
                p.push(idx_in_parent - 1);
                p
            };
            let prev_id = match self.get_node_by_path_readonly(&prev_sibling_path) { Some(n) => n.node.id.clone(), None => return Ok(()) };
            // Move selected under previous sibling at end
            let selected_id = self.get_node_by_path_readonly(path).map(|n| n.node.id.clone()).unwrap();
            let note_id = self.current_note.as_ref().map(|n| n.id.clone()).unwrap_or_default();
            let next_pos = NodeRepository::get_next_child_position(&self.db_connection, Some(&prev_id), &note_id)?;
            NodeRepository::update_parent_and_position(&self.db_connection, &selected_id, Some(&prev_id), next_pos)?;
            self.refresh_current_note_preserve_selection(Some(&selected_id))?;
        }
        Ok(())
    }

    /// Outdent the selected node (move it to parent's parent, after parent)
    pub fn outdent_selected(&mut self) -> Result<()> {
        let paths = self.build_visible_paths();
        if let Some(path) = paths.get(self.cursor_position) {
            if path.len() < 2 { return Ok(()); }
            // Parent path and grandparent path
            let _parent_path = &path[..path.len()-1];
            let grandparent_path = &path[..path.len()-2];
            let grandparent_id_opt = if grandparent_path.is_empty() { None } else { self.get_node_by_path_readonly(grandparent_path).map(|n| n.node.id.clone()) };
            let selected_id = self.get_node_by_path_readonly(path).map(|n| n.node.id.clone()).unwrap();
            let note_id = self.current_note.as_ref().map(|n| n.id.clone()).unwrap_or_default();
            // New position is after the parent among its siblings
            let new_pos = if let Some(grand_id) = &grandparent_id_opt {
                let next = NodeRepository::get_next_child_position(&self.db_connection, Some(grand_id), &note_id)?;
                next
            } else {
                NodeRepository::get_next_child_position(&self.db_connection, None, &note_id)?
            };
            NodeRepository::update_parent_and_position(&self.db_connection, &selected_id, grandparent_id_opt.as_deref(), new_pos)?;
            self.refresh_current_note_preserve_selection(Some(&selected_id))?;
        }
        Ok(())
    }

    /// Move selected node up among siblings
    pub fn move_selected_up(&mut self) -> Result<()> {
        let paths = self.build_visible_paths();
        if let Some(path) = paths.get(self.cursor_position) {
            if path.is_empty() { return Ok(()); }
            let idx_in_parent = *path.last().unwrap();
            if idx_in_parent == 0 { return Ok(()); }
            let parent_path = &path[..path.len()-1];
            let current_id = self.get_node_by_path_readonly(path).map(|n| n.node.id.clone()).unwrap();
            let prev_path = {
                let mut p = parent_path.to_vec();
                p.push(idx_in_parent - 1);
                p
            };
            let prev_id = self.get_node_by_path_readonly(&prev_path).map(|n| n.node.id.clone()).unwrap();
            NodeRepository::swap_positions(&self.db_connection, &current_id, &prev_id)?;
            self.refresh_current_note_preserve_selection(Some(&current_id))?;
        }
        Ok(())
    }

    /// Move selected node down among siblings
    pub fn move_selected_down(&mut self) -> Result<()> {
        let paths = self.build_visible_paths();
        if let Some(path) = paths.get(self.cursor_position) {
            if path.is_empty() { return Ok(()); }
            let parent_path = &path[..path.len()-1];
            let idx_in_parent = *path.last().unwrap();
            let siblings_count = self.get_children_count_by_path(parent_path);
            if idx_in_parent + 1 >= siblings_count { return Ok(()); }
            let current_id = self.get_node_by_path_readonly(path).map(|n| n.node.id.clone()).unwrap();
            let next_path = {
                let mut p = parent_path.to_vec();
                p.push(idx_in_parent + 1);
                p
            };
            let next_id = self.get_node_by_path_readonly(&next_path).map(|n| n.node.id.clone()).unwrap();
            NodeRepository::swap_positions(&self.db_connection, &current_id, &next_id)?;
            self.refresh_current_note_preserve_selection(Some(&current_id))?;
        }
        Ok(())
    }

    fn get_children_count_by_path(&self, parent_path: &[usize]) -> usize {
        if parent_path.is_empty() { return self.outline_tree.len(); }
        self.get_node_by_path_readonly(parent_path).map(|n| n.children.len()).unwrap_or(0)
    }

    fn get_node_by_path_readonly(&self, path: &[usize]) -> Option<&TreeNode> {
        if path.is_empty() { return None; }
        let mut node = self.outline_tree.get(path[0])?;
        for idx in &path[1..] {
            node = node.children.get(*idx)?;
        }
        Some(node)
    }

    /// Reload current note's tree from DB and try to preserve selection by node id
    pub fn refresh_current_note_preserve_selection(&mut self, prefer_id: Option<&str>) -> Result<()> {
        if let Some(note) = &self.current_note {
            let nodes = NodeRepository::get_by_note_id(&self.db_connection, &note.id)?;
            self.outline_tree = TreeNode::build_tree(nodes);
            // Determine preferred target id as owned String to avoid lifetime issues

            // Refresh attachments for current note
            self.refresh_attachments()?;
            let preferred: Option<String> = match prefer_id {
                Some(id) => Some(id.to_string()),
                None => self.get_selected_node_id(),
            };

            if let Some(target_id) = preferred {
                // Find index of target_id in new visible listing
                let visible = self.get_visible_nodes();
                if let Some(new_idx) = visible.iter().position(|t| t.node.id == target_id) {
                    self.cursor_position = new_idx;
                } else {
                    self.cursor_position = 0;
                }
            } else {
                self.cursor_position = 0;
            }
        }
        Ok(())
    }

    /// Handle tick events
    pub fn tick(&mut self) {
        // Future: periodic updates, autosave, etc.
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    // =========================
    // Phase 4: Pages management
    // =========================

    /// Refresh the cached list of notes for pages UI
    pub fn refresh_notes_list(&mut self) -> Result<()> {
        self.notes = NoteRepository::get_all(&self.db_connection)?;
        // Apply tag filter if present (Phase 5)
        if let Some(tag_name) = &self.tag_filter {
            let note_ids = TagRepository::get_note_ids_for_tag_name(&self.db_connection, tag_name)?;
            self.notes.retain(|n| note_ids.iter().any(|id| *id == n.id));
        }
        // Keep sidebar selection aligned with current note if possible
        if let Some(current) = &self.current_note {
            if let Some(idx) = self.notes.iter().position(|n| n.id == current.id) {
                self.sidebar_pages_selected_index = idx;
            }
        }
        // Refresh favorites
        self.favorites = FavoriteRepository::get_all(&self.db_connection)?;
        Ok(())
    }

    // =========================
    // Phase 5: Search
    // =========================
    pub fn open_search(&mut self) {
        self.search_open = true;
        self.search_query.clear();
        self.search_results.clear();
        self.search_selection = 0;
    }

    pub fn close_search(&mut self) {
        self.search_open = false;
        self.search_query.clear();
        self.search_results.clear();
        self.search_selection = 0;
    }

    pub fn perform_search(&mut self) -> Result<()> {
        if self.search_query.is_empty() {
            self.search_results.clear();
        } else {
            self.search_results = NodeRepository::search(&self.db_connection, &self.search_query)?;
        }
        self.search_selection = 0;
        self.search_open = false; // Close search bar, show results
        Ok(())
    }

    pub fn search_results_up(&mut self) {
        if !self.search_results.is_empty() {
            self.search_selection = self.search_selection.saturating_sub(1);
        }
    }

    pub fn search_results_down(&mut self) {
        if !self.search_results.is_empty() {
            let max = self.search_results.len() - 1;
            if self.search_selection < max {
                self.search_selection += 1;
            }
        }
    }

    pub fn search_results_select(&mut self) -> Result<()> {
        if let Some(node) = self.search_results.get(self.search_selection) {
            self.load_note(&node.note_id)?;
            // Find the node in the visible nodes and set cursor
            let visible = self.get_visible_nodes();
            if let Some(idx) = visible.iter().position(|t| t.node.id == node.id) {
                self.cursor_position = idx;
            }
        }
        self.search_results.clear();
        self.search_selection = 0;
        Ok(())
    }

    pub fn update_search_query(&mut self, ch: char) {
        self.search_query.push(ch);
        self.run_search();
    }

    pub fn backspace_search_query(&mut self) {
        self.search_query.pop();
        self.run_search();
    }

    pub fn run_search(&mut self) {
        if self.search_query.trim().is_empty() {
            self.search_results.clear();
            return;
        }
        if let Ok(results) = NodeRepository::search(&self.db_connection, &self.search_query) {
            self.search_results = results;
        }
    }

    // =========================
    // Phase 5: Tags filter
    // =========================
    pub fn clear_tag_filter(&mut self) -> Result<()> {
        self.tag_filter = None;
        self.refresh_notes_list()
    }

    pub fn set_tag_filter(&mut self, tag_name: String) -> Result<()> {
        self.tag_filter = Some(tag_name);
        self.refresh_notes_list()
    }

    pub fn select_favorite_by_index(&mut self, index: usize) -> Result<()> {
        if index < self.favorites.len() {
            let id = self.favorites[index].note_id.clone();
            self.load_note(&id)?;
        }
        Ok(())
    }

    /// Select a page by index from `notes`
    pub fn select_page_by_index(&mut self, index: usize) -> Result<()> {
        if index < self.notes.len() {
            let id = self.notes[index].id.clone();
            self.sidebar_pages_selected_index = index;
            self.load_note(&id)?;
        }
        Ok(())
    }

    /// Create a new page with a generated title and switch to it
    pub fn create_new_page(&mut self) -> Result<()> {
        // Generate a unique title like "Untitled" or "Untitled (n)"
        let base = "Untitled".to_string();
        let mut title = base.clone();
        let mut suffix = 1;
        let existing_titles: std::collections::HashSet<String> = self
            .notes
            .iter()
            .map(|n| n.title.to_lowercase())
            .collect();
        while existing_titles.contains(&title.to_lowercase()) {
            title = format!("{} ({})", base, suffix);
            suffix += 1;
        }

        let note = Note::new(title);
        NoteRepository::create(&self.db_connection, &note)?;
        self.refresh_notes_list()?;
        if let Some(idx) = self.notes.iter().position(|n| n.id == note.id) {
            self.select_page_by_index(idx)?;
        }
        Ok(())
    }

    /// Delete the current page; if none remain, create a new default
    pub fn delete_current_page(&mut self) -> Result<()> {
        let current_id = match &self.current_note { Some(n) => n.id.clone(), None => return Ok(()) };
        NoteRepository::delete(&self.db_connection, &current_id)?;
        self.refresh_notes_list()?;
        if self.notes.is_empty() {
            // Ensure at least one page exists
            let note = Note::new("Welcome".to_string());
            NoteRepository::create(&self.db_connection, &note)?;
            self.refresh_notes_list()?;
        }
        // Load first note or keep index if valid
        let idx = self.sidebar_pages_selected_index.min(self.notes.len().saturating_sub(1));
        if !self.notes.is_empty() {
            self.select_page_by_index(idx)?;
        }
        Ok(())
    }

    /// Navigate sidebar page selection up
    pub fn sidebar_select_up(&mut self) {
        if self.sidebar_pages_selected_index > 0 {
            self.sidebar_pages_selected_index -= 1;
        }
    }

    /// Navigate sidebar page selection down
    pub fn sidebar_select_down(&mut self) {
        let last = self.notes.len().saturating_sub(1);
        if self.sidebar_pages_selected_index < last {
            self.sidebar_pages_selected_index += 1;
        }
    }

    /// Activate the sidebar-selected page
    pub fn sidebar_activate_selected(&mut self) -> Result<()> {
        self.select_page_by_index(self.sidebar_pages_selected_index)
    }

    /// Open the page switcher overlay
    pub fn open_page_switcher(&mut self) -> Result<()> {
        self.page_switcher_open = true;
        self.page_filter.clear();
        self.page_switcher_selection_index = 0;
        // Ensure notes list is up to date
        self.refresh_notes_list()?;
        Ok(())
    }

    /// Close the page switcher overlay
    pub fn close_page_switcher(&mut self) {
        self.page_switcher_open = false;
        self.page_filter.clear();
        self.page_switcher_selection_index = 0;
    }

    /// Get filtered notes based on the current page filter (substring, case-insensitive)
    pub fn get_filtered_notes(&self) -> Vec<&Note> {
        if self.page_filter.is_empty() {
            return self.notes.iter().collect();
        }
        let needle = self.page_filter.to_lowercase();
        self
            .notes
            .iter()
            .filter(|n| n.title.to_lowercase().contains(&needle))
            .collect()
    }

    /// Move selection in page switcher up
    pub fn page_switcher_up(&mut self) {
        if self.page_switcher_selection_index > 0 {
            self.page_switcher_selection_index -= 1;
        }
    }

    /// Move selection in page switcher down
    pub fn page_switcher_down(&mut self) {
        let last = self.get_filtered_notes().len().saturating_sub(1);
        if self.page_switcher_selection_index < last {
            self.page_switcher_selection_index += 1;
        }
    }

    /// Apply the current selection in page switcher
    pub fn page_switcher_activate(&mut self) -> Result<()> {
        let filtered = self.get_filtered_notes();
        if let Some(note) = filtered.get(self.page_switcher_selection_index) {
            // Take copies before mutable borrows
            let selected_id = note.id.clone();
            let sidebar_idx = self.notes.iter().position(|n| n.id == selected_id);
            if let Some(idx) = sidebar_idx { self.sidebar_pages_selected_index = idx; }
            self.load_note(&selected_id)?;
        }
        self.close_page_switcher();
        Ok(())
    }

    // Favorites operations
    pub fn toggle_favorite_current(&mut self) -> Result<()> {
        if let Some(current) = &self.current_note {
            if FavoriteRepository::is_favorited(&self.db_connection, &current.id)? {
                let _ = FavoriteRepository::delete(&self.db_connection, &current.id)?;
            } else {
                let pos = FavoriteRepository::get_next_position(&self.db_connection)?;
                let fav = notiq_core::models::Favorite::new(current.id.clone(), pos);
                FavoriteRepository::create(&self.db_connection, &fav)?;
            }
            self.favorites = FavoriteRepository::get_all(&self.db_connection)?;
        }
        Ok(())
    }

    pub fn open_logbook_for_selected(&mut self) -> Result<()> {
        if let Some(node_id) = self.get_selected_node_id() {
            self.logbook_entries = TaskLogRepository::get_by_node_id(&self.db_connection, &node_id)?;
            self.logbook_open = true;
        }
        Ok(())
    }

    pub fn close_logbook(&mut self) {
        self.logbook_open = false;
        self.logbook_entries.clear();
    }

    pub fn export_markdown(&mut self, out_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(out_dir)?;
        // Export notes as simple files
        for note in NoteRepository::get_all(&self.db_connection)? {
            let nodes = NodeRepository::get_by_note_id(&self.db_connection, &note.id)?;
            let mut content = String::new();
            content.push_str(&format!("# {}\n\n", note.title));
            for n in nodes {
                let indent = "  ".repeat(Self::node_depth(&self.outline_tree, &n.id).unwrap_or(0));
                content.push_str(&format!("{}- {}\n", indent, n.content));
            }
            let safe = note.title.replace('/', "-");
            let path = out_dir.join(format!("{}.md", safe));
            std::fs::write(path, content)?;
        }
        Ok(())
    }

    fn node_depth(tree: &Vec<TreeNode>, node_id: &str) -> Option<usize> {
        fn walk<'a>(t: &'a TreeNode, id: &str) -> Option<usize> {
            if t.node.id == id { return Some(t.depth); }
            for c in &t.children { if let Some(d) = walk(c, id) { return Some(d); } }
            None
        }
        for t in tree { if let Some(d) = walk(t, node_id) { return Some(d); } }
        None
    }

    /// Simple input debounce to avoid double-processing on some terminals
    pub fn should_accept_input(&mut self, min_interval_ms: u64) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_input_time {
            if now.duration_since(last).as_millis() < (min_interval_ms as u128) {
                return false;
            }
        }
        self.last_input_time = Some(now);
        true
    }

    pub fn toggle_sidebar(&mut self) {
        self.show_sidebar = !self.show_sidebar;
    }

    // =========================
    // Phase 7: Attachments helpers
    // =========================
    fn attachments_dir(&self) -> PathBuf {
        self.workspace_dir.join("attachments")
    }

    pub fn refresh_attachments(&mut self) -> Result<()> {
        if let Some(note) = &self.current_note {
            self.attachments = AttachmentRepository::get_by_note_id(&self.db_connection, &note.id)?;
            if self.attachments_selected_index >= self.attachments.len() {
                self.attachments_selected_index = self.attachments.len().saturating_sub(1);
            }
        } else {
            self.attachments.clear();
            self.attachments_selected_index = 0;
        }
        Ok(())
    }

    pub fn open_attachments_overlay(&mut self) {
        self.attach_overlay_open = true;
        self.attach_input.clear();
    }

    pub fn close_attachments_overlay(&mut self) {
        self.attach_overlay_open = false;
        self.attach_input.clear();
    }

    pub fn update_attach_input(&mut self, ch: char) {
        self.attach_input.push(ch);
    }

    pub fn backspace_attach_input(&mut self) {
        self.attach_input.pop();
    }

    pub fn confirm_attach(&mut self) -> Result<()> {
        let path = self.attach_input.trim().to_string();
        if !path.is_empty() {
            self.attach_file_from_path(Path::new(&path))?;
        }
        self.close_attachments_overlay();
        Ok(())
    }

    pub fn attachments_select_up(&mut self) {
        if self.attachments_selected_index > 0 {
            self.attachments_selected_index -= 1;
        }
    }

    pub fn attachments_select_down(&mut self) {
        let last = self.attachments.len().saturating_sub(1);
        if self.attachments_selected_index < last {
            self.attachments_selected_index += 1;
        }
    }

    pub fn open_selected_attachment(&mut self) -> Result<()> {
        if self.attachments.is_empty() { return Ok(()); }
        let att = &self.attachments[self.attachments_selected_index];
        let path = Path::new(&att.filepath);
        let _ = opener::open(path);
        Ok(())
    }

    fn attach_file_from_path(&mut self, src_path: &Path) -> Result<()> {
        // Validate source file
        let metadata = std::fs::metadata(src_path)?;
        if !metadata.is_file() { return Ok(()); }

        // Compute SHA-256 hash
        let mut file = std::fs::File::open(src_path)?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 8192];
        loop {
            let read = file.read(&mut buf)?;
            if read == 0 { break; }
            hasher.update(&buf[..read]);
        }
        let hash_bytes = hasher.finalize();
        let hash_hex = hex::encode(hash_bytes);

        // Determine destination path (hash + original extension)
        let ext = src_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let filename_hashed = if ext.is_empty() { hash_hex.clone() } else { format!("{}.{}", hash_hex, ext) };
        
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let attachments_dir = self.attachments_dir().join(today);

        std::fs::create_dir_all(&attachments_dir)?;
        let dest_path = attachments_dir.join(&filename_hashed);

        // If a file with same hash exists, reuse; else copy
        if !dest_path.exists() {
            std::fs::copy(src_path, &dest_path)?;
        }

        // MIME type guess
        let mime = mime_guess::from_path(src_path).first_raw().map(|s| s.to_string());
        let size_bytes = metadata.len() as i64;

        // Create DB record
        let note_id = match &self.current_note { Some(n) => n.id.clone(), None => return Ok(()) };
        let node_id = match self.get_selected_node() {
            Some(n) => n.node.id.clone(),
            None => {
                // If there's no selected node, maybe there are no nodes. Create one.
                if self.get_visible_nodes().is_empty() {
                    let new_node = notiq_core::models::OutlineNode::new(note_id.clone(), None, "".to_string(), 0);
                    NodeRepository::create(&self.db_connection, &new_node)?;
                    self.refresh_current_note()?;
                    new_node.id.clone()
                } else {
                    return Ok(()); // Or handle this case appropriately
                }
            }
        };

        let attachment = Attachment::new(
            note_id,
            node_id,
            src_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
            dest_path.to_string_lossy().to_string(),
            mime,
            size_bytes,
            hash_hex,
        );
        AttachmentRepository::create(&self.db_connection, &attachment)?;
        self.refresh_attachments()?;
        Ok(())
    }

    // =========================
    // Autocomplete methods
    // =========================
    
    /// Check if we should trigger autocomplete based on the current edit buffer
    pub fn check_autocomplete_trigger(&mut self) {
        if !self.is_editing {
            self.close_autocomplete();
            return;
        }

        let text = &self.edit_buffer;
        
        // Check for [[ wiki link trigger
        if let Some(pos) = text.rfind("[[") {
            let after = &text[pos+2..];
            // Only trigger if there's no closing ]]
            if !after.contains("]]") {
                self.autocomplete_type = AutocompleteType::WikiLink;
                self.autocomplete_trigger_pos = pos;
                self.autocomplete_items = self.get_note_titles();
                self.autocomplete_selection = 0;
                self.autocomplete_open = true;
                return;
            }
        }
        
        // Check for # tag trigger - find last # that starts a word
        if let Some(pos) = text.rfind(|c: char| c == '#') {
            let after = &text[pos+1..];
            // Only trigger if the character after # is alphanumeric or empty (no whitespace)
            if after.is_empty() || after.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                // Also check that # is at start or preceded by whitespace
                let before_ok = pos == 0 || text.chars().nth(pos.saturating_sub(1)).map(|c| c.is_whitespace()).unwrap_or(true);
                if before_ok {
                    self.autocomplete_type = AutocompleteType::Tag;
                    self.autocomplete_trigger_pos = pos;
                    self.autocomplete_items = self.get_tag_names();
                    self.autocomplete_selection = 0;
                    self.autocomplete_open = true;
                    return;
                }
            }
        }
        
        // Close autocomplete if no trigger found
        self.close_autocomplete();
    }
    
    fn get_note_titles(&self) -> Vec<String> {
        self.notes.iter().map(|n| n.title.clone()).collect()
    }
    
    fn get_tag_names(&self) -> Vec<String> {
        TagRepository::get_usage_counts(&self.db_connection)
            .ok()
            .map(|counts| counts.into_iter().map(|(tag, _)| tag.name).collect())
            .unwrap_or_default()
    }
    
    pub fn close_autocomplete(&mut self) {
        self.autocomplete_open = false;
        self.autocomplete_type = AutocompleteType::None;
        self.autocomplete_items.clear();
        self.autocomplete_selection = 0;
    }
    
    pub fn autocomplete_up(&mut self) {
        if self.autocomplete_selection > 0 {
            self.autocomplete_selection -= 1;
        }
    }
    
    pub fn autocomplete_down(&mut self) {
        if self.autocomplete_selection < self.autocomplete_items.len().saturating_sub(1) {
            self.autocomplete_selection += 1;
        }
    }
    
    pub fn autocomplete_select(&mut self) -> Result<()> {
        if !self.autocomplete_open || self.autocomplete_items.is_empty() {
            return Ok(());
        }
        
        let selected = self.autocomplete_items[self.autocomplete_selection].clone();
        let trigger_pos = self.autocomplete_trigger_pos;
        
        match self.autocomplete_type {
            AutocompleteType::WikiLink => {
                // Replace from [[ onwards with [[selected]]
                self.edit_buffer.truncate(trigger_pos);
                self.edit_buffer.push_str(&format!("[[{}]]", selected));
            }
            AutocompleteType::Tag => {
                // Replace from # onwards with #selected
                self.edit_buffer.truncate(trigger_pos);
                self.edit_buffer.push_str(&format!("#{}", selected));
            }
            AutocompleteType::None => {}
        }
        
        self.close_autocomplete();
        Ok(())
    }

    // =========================
    // Page renaming methods
    // =========================
    
    pub fn start_renaming_page(&mut self) {
        if let Some(note) = &self.current_note {
            self.is_renaming_page = true;
            self.page_title_buffer = note.title.clone();
        }
    }
    
    pub fn cancel_page_rename(&mut self) {
        self.is_renaming_page = false;
        self.page_title_buffer.clear();
    }
    
    pub fn commit_page_rename(&mut self) -> Result<()> {
        if !self.is_renaming_page {
            return Ok(());
        }
        
        if let Some(mut note) = self.current_note.clone() {
            note.title = self.page_title_buffer.clone();
            note.touch();
            NoteRepository::update(&self.db_connection, &note)?;
            
            // Refresh current note and the list of all notes
            self.current_note = Some(note);
            self.refresh_notes_list()?;
        }
        
        self.cancel_page_rename();
        Ok(())
    }

    // =========================
    // Task overview methods
    // =========================
    
    pub fn open_task_overview(&mut self) {
        self.task_overview_open = true;
        self.task_overview_selection = 0;
        self.refresh_task_overview();
    }
    
    pub fn close_task_overview(&mut self) {
        self.task_overview_open = false;
        self.task_overview_tasks.clear();
    }
    
    fn refresh_task_overview(&mut self) {
        self.task_overview_tasks.clear();
        
        // Get all notes
        if let Ok(notes) = NoteRepository::get_all(&self.db_connection) {
            for note in notes {
                // Get all nodes for this note
                if let Ok(nodes) = NodeRepository::get_by_note_id(&self.db_connection, &note.id) {
                    for node in nodes {
                        if node.is_task {
                            self.task_overview_tasks.push(TaskOverviewItem {
                                node,
                                note_title: note.title.clone(),
                                note_id: note.id.clone(),
                            });
                        }
                    }
                }
            }
        }
        
        // Sort by priority and completion status
        self.task_overview_tasks.sort_by(|a, b| {
            // Uncompleted tasks first
            match (a.node.task_completed, b.node.task_completed) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => {
                    // Then by priority
                    match (&a.node.task_priority, &b.node.task_priority) {
                        (Some(notiq_core::models::TaskPriority::High), _) => std::cmp::Ordering::Less,
                        (_, Some(notiq_core::models::TaskPriority::High)) => std::cmp::Ordering::Greater,
                        (Some(notiq_core::models::TaskPriority::Medium), Some(notiq_core::models::TaskPriority::Low)) => std::cmp::Ordering::Less,
                        (Some(notiq_core::models::TaskPriority::Low), Some(notiq_core::models::TaskPriority::Medium)) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    }
                }
            }
        });
    }
    
    pub fn task_overview_up(&mut self) {
        if self.task_overview_selection > 0 {
            self.task_overview_selection -= 1;
        }
    }
    
    pub fn task_overview_down(&mut self) {
        if self.task_overview_selection < self.task_overview_tasks.len().saturating_sub(1) {
            self.task_overview_selection += 1;
        }
    }
    
    pub fn task_overview_toggle_selected(&mut self) -> Result<()> {
        if self.task_overview_tasks.is_empty() {
            return Ok(());
        }
        
        let task_item = &self.task_overview_tasks[self.task_overview_selection];
        let node_id = task_item.node.id.clone();
        
        // Toggle the task
        let mut node = NodeRepository::get_by_id(&self.db_connection, &node_id)?;
        let old = node.task_completed;
        let now_completed = node.toggle_task();
        NodeRepository::update(&self.db_connection, &node)?;
        
        // Log status change
        let status = if now_completed { TaskStatus::Completed } else { TaskStatus::Uncompleted };
        let log = TaskStatusLog::new(
            node.id.clone(),
            status,
            Some(old.to_string()),
            Some(now_completed.to_string()),
        );
        let _ = TaskLogRepository::create(&self.db_connection, &log)?;
        
        // Refresh the task overview
        self.refresh_task_overview();
        
        Ok(())
    }
    
    pub fn task_overview_goto_selected(&mut self) -> Result<()> {
        if self.task_overview_tasks.is_empty() {
            return Ok(());
        }
        
        let task_item = &self.task_overview_tasks[self.task_overview_selection];
        let note_id = task_item.note_id.clone();
        let node_id = task_item.node.id.clone();
        
        // Load the note
        self.load_note(&note_id)?;
        
        // Find the node in visible nodes and set cursor
        let visible = self.get_visible_nodes();
        if let Some(idx) = visible.iter().position(|t| t.node.id == node_id) {
            self.cursor_position = idx;
        }
        
        self.close_task_overview();
        Ok(())
    }

    // =========================
    // Calendar click support
    // =========================
    
    pub fn calendar_click_day(&mut self, row: usize, col: usize) -> Result<()> {
        let month_start = self.calendar_month_start;
        let first_weekday = month_start.weekday().num_days_from_monday() as usize;
        
        let cell_index = row * 7 + col;
        if cell_index < first_weekday {
            return Ok(());
        }
        
        let day = (cell_index - first_weekday + 1) as u32;
        let days_in_month = days_in_month(month_start.year(), month_start.month());
        
        if day > days_in_month {
            return Ok(());
        }
        
        if let Some(date) = NaiveDate::from_ymd_opt(month_start.year(), month_start.month(), day) {
            self.calendar_selected = date;
            // Optionally auto-open the daily note
            self.open_selected_daily_note()?;
        }
        
        Ok(())
    }

    // =========================
    // Clipboard support
    // =========================
    
    pub fn paste_from_clipboard(&mut self) -> Result<()> {
        // Try to get clipboard contents
        #[cfg(feature = "clipboard")]
        {
            use arboard::Clipboard;
            let mut clipboard = Clipboard::new()?;
            
            // Check if clipboard has an image
            if let Ok(img) = clipboard.get_image() {
                // Save image to temp file, then attach
                let temp_dir = std::env::temp_dir();
                let timestamp = chrono::Utc::now().timestamp();
                let temp_path = temp_dir.join(format!("pasted_image_{}.png", timestamp));
                
                // Convert image to PNG and save
                let img_data = img.bytes;
                std::fs::write(&temp_path, img_data)?;
                
                self.attach_file_from_path(&temp_path)?;
                
                // Clean up temp file
                let _ = std::fs::remove_file(&temp_path);
                
                return Ok(());
            }
            
            // If not an image, try text
            if let Ok(text) = clipboard.get_text() {
                if self.is_editing {
                    let current_pos = self.edit_cursor_position;
                    let byte_pos = self.edit_buffer.char_indices().map(|(i, _)| i).nth(current_pos).unwrap_or(self.edit_buffer.len());
                    self.edit_buffer.insert_str(byte_pos, &text);
                    self.edit_cursor_position += text.chars().count();
                }
            }
        }
        
        Ok(())
    }

    /// Open the help screen
    pub fn open_help(&mut self) {
        self.help_open = true;
    }

    /// Close the help screen
    pub fn close_help(&mut self) {
        self.help_open = false;
    }

    /// Create a quote block below the current selection
    pub fn create_quote_block(&mut self) -> Result<()> {
        self.create_special_block(notiq_core::models::BlockType::Quote, "> ")
    }

    /// Create a code block below the current selection
    pub fn create_code_block(&mut self) -> Result<()> {
        self.create_special_block(notiq_core::models::BlockType::Code, "```\n\n```")
    }

    /// Create a special block (quote or code) below the current selection
    fn create_special_block(&mut self, block_type: notiq_core::models::BlockType, default_content: &str) -> Result<()> {
        let note_id = match &self.current_note { Some(n) => n.id.clone(), None => return Ok(()) };
        let selected_paths = self.build_visible_paths();

        if selected_paths.is_empty() {
            // No nodes on page, create a new root block.
            let next_pos = NodeRepository::get_next_child_position(&self.db_connection, None, &note_id)?;
            let new_node = OutlineNode::new_block(note_id, None, default_content.to_string(), next_pos, block_type);
            let new_id = new_node.id.clone();
            NodeRepository::create(&self.db_connection, &new_node)?;
            self.refresh_current_note_preserve_selection(Some(&new_id))?;
            self.start_editing();
        } else if let Some(path) = selected_paths.get(self.cursor_position) {
            // Determine parent of selected
            let parent_id_opt = if path.len() == 1 {
                None
            } else {
                // Get parent path
                let parent_path = &path[..path.len()-1];
                let parent_node = self.get_node_by_path_readonly(parent_path);
                parent_node.map(|n| n.node.id.clone())
            };

            // Next position among siblings
            let next_pos = NodeRepository::get_next_child_position(
                &self.db_connection,
                parent_id_opt.as_deref(),
                &note_id,
            )?;

            let new_node = OutlineNode::new_block(note_id, parent_id_opt, default_content.to_string(), next_pos, block_type);
            let new_id = new_node.id.clone();
            NodeRepository::create(&self.db_connection, &new_node)?;
            self.refresh_current_note_preserve_selection(Some(&new_id))?;
            self.start_editing();
        }
        Ok(())
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    let last_this = first_next - chrono::Duration::days(1);
    last_this.day()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_tree_node_build() {
        let nodes = vec![
            OutlineNode::new("note1".to_string(), None, "Root".to_string(), 0),
            OutlineNode::new("note1".to_string(), Some("parent".to_string()), "Child".to_string(), 1),
        ];

        // Can't fully test without proper parent IDs, but structure is valid
        let tree = TreeNode::build_tree(nodes);
        assert!(!tree.is_empty());
    }

    #[test]
    fn test_app_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let app = App::new(db_path.to_str().unwrap()).unwrap();
        assert!(!app.should_quit);
        assert!(app.current_note.is_none());
    }

    #[test]
    fn test_initialize_and_load() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        
        let mut app = App::new(db_path.to_str().unwrap()).unwrap();
        app.initialize_sample_data().unwrap();
        app.load_first_note().unwrap();
        
        assert!(app.current_note.is_some());
        assert!(!app.outline_tree.is_empty());
    }
}

