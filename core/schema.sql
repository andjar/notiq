-- Outliner Application SQLite Schema
-- Version: 1.0.0

-- Core notes table (each note is a page/document)
CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL
);

-- Create index for title searches
CREATE INDEX IF NOT EXISTS idx_notes_title ON notes(title);
CREATE INDEX IF NOT EXISTS idx_notes_modified ON notes(modified_at DESC);

-- Outliner structure (nodes)
CREATE TABLE IF NOT EXISTS outline_nodes (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    parent_node_id TEXT,
    content TEXT NOT NULL,
    position INTEGER NOT NULL, -- for ordering siblings
    is_task BOOLEAN DEFAULT 0,
    task_completed BOOLEAN DEFAULT 0,
    task_priority TEXT, -- 'low', 'medium', 'high'
    task_due_date INTEGER,
    block_type TEXT DEFAULT 'normal', -- 'normal', 'quote', 'code'
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY(parent_node_id) REFERENCES outline_nodes(id) ON DELETE CASCADE
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_outline_nodes_note_id ON outline_nodes(note_id);
CREATE INDEX IF NOT EXISTS idx_outline_nodes_parent ON outline_nodes(parent_node_id);
CREATE INDEX IF NOT EXISTS idx_outline_nodes_position ON outline_nodes(note_id, parent_node_id, position);
CREATE INDEX IF NOT EXISTS idx_outline_nodes_tasks ON outline_nodes(is_task, task_completed);

-- Full-text search for outline nodes
CREATE VIRTUAL TABLE IF NOT EXISTS nodes_fts USING fts5(
    node_id UNINDEXED,
    content,
    content='outline_nodes',
    content_rowid='id',
    tokenize='porter'
);

-- Triggers to keep FTS index in sync
CREATE TRIGGER IF NOT EXISTS nodes_fts_insert AFTER INSERT ON outline_nodes BEGIN
    INSERT INTO nodes_fts(rowid, node_id, content)
    VALUES (new.rowid, new.id, new.content);
END;

CREATE TRIGGER IF NOT EXISTS nodes_fts_delete AFTER DELETE ON outline_nodes BEGIN
    INSERT INTO nodes_fts(nodes_fts, rowid, node_id, content)
    VALUES ('delete', old.rowid, old.id, old.content);
END;

CREATE TRIGGER IF NOT EXISTS nodes_fts_update AFTER UPDATE ON outline_nodes BEGIN
    INSERT INTO nodes_fts(nodes_fts, rowid, node_id, content)
    VALUES ('delete', old.rowid, old.id, old.content);
    INSERT INTO nodes_fts(rowid, node_id, content)
    VALUES (new.rowid, new.id, new.content);
END;

-- Tags
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    color TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);

-- Tag associations with nodes
CREATE TABLE IF NOT EXISTS node_tags (
    node_id TEXT NOT NULL,
    tag_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (node_id, tag_id),
    FOREIGN KEY(node_id) REFERENCES outline_nodes(id) ON DELETE CASCADE,
    FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_node_tags_tag_id ON node_tags(tag_id);

-- Links (bidirectional references between notes)
CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_note_id TEXT NOT NULL,
    source_node_id TEXT,
    target_note_id TEXT NOT NULL,
    link_text TEXT,
    link_type TEXT NOT NULL, -- 'wiki', 'transclusion', 'attachment'
    created_at INTEGER NOT NULL,
    FOREIGN KEY(source_note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY(source_node_id) REFERENCES outline_nodes(id) ON DELETE CASCADE
    -- target_note_id may not exist yet, so no FK constraint
);

CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_note_id);
CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_note_id);
CREATE INDEX IF NOT EXISTS idx_links_type ON links(link_type);

-- Attachments
CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    note_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    filepath TEXT NOT NULL,
    mime_type TEXT,
    size_bytes INTEGER NOT NULL,
    hash TEXT NOT NULL, -- for deduplication
    created_at INTEGER NOT NULL,
    FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY(node_id) REFERENCES outline_nodes(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_attachments_note_id ON attachments(note_id);
CREATE INDEX IF NOT EXISTS idx_attachments_hash ON attachments(hash);

-- Daily notes index
CREATE TABLE IF NOT EXISTS daily_notes (
    date TEXT PRIMARY KEY, -- YYYY-MM-DD
    note_id TEXT UNIQUE NOT NULL,
    FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);

-- Favorites
CREATE TABLE IF NOT EXISTS favorites (
    note_id TEXT PRIMARY KEY,
    position INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_favorites_position ON favorites(position);

-- Log of task status changes
CREATE TABLE IF NOT EXISTS task_status_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    status TEXT NOT NULL, -- 'created', 'completed', 'uncompleted', 'deleted'
    old_value TEXT,
    new_value TEXT,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY(node_id) REFERENCES outline_nodes(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_task_log_node_id ON task_status_log(node_id);
CREATE INDEX IF NOT EXISTS idx_task_log_timestamp ON task_status_log(timestamp DESC);

-- Application metadata
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Insert schema version
INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', '1');
INSERT OR REPLACE INTO metadata (key, value) VALUES ('created_at', strftime('%s', 'now'));

