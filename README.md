# Notiq - A Blazing-Fast Cross-Platform Outliner

A powerful, keyboard-first outliner application with infinite nesting, tags, task management, linking/backlinking, and file attachments. Built with Rust for speed and reliability.

## Features

- **Infinite nesting outliner** with expand/collapse
- **Tags system** with filtering and organization
- **Task management** with checkboxes, due dates, and priorities
- **Bidirectional linking** (wiki-style [[links]] and automatic backlinks)
- **Transclusion** (embed content from other notes)
- **File attachments** with deduplication
- **Journal/daily notes** interface
- **Full-text search** with SQLite FTS5
- **Page-based organization**
- **Keyboard-first workflow** with mouse support

## Project Status

✅ **Phase 1 Complete**: Core data model and storage layer
- ✅ Cargo workspace setup
- ✅ SQLite schema with FTS5 support
- ✅ Complete data models (Note, OutlineNode, Tag, Link, Attachment, etc.)
- ✅ Database initialization and migration logic
- ✅ Repository layer with full CRUD operations
- ✅ Comprehensive unit tests

✅ **Phase 2 Complete**: Basic TUI and read-only outliner
- ✅ Working TUI application with Ratatui
- ✅ Hierarchical outline rendering with proper indentation
- ✅ Tree data structure for nested content
- ✅ Event handling and keyboard input
- ✅ Visual distinction for tasks, priorities, and node types
- ✅ Sample data initialization

✅ **Phase 3 Complete**: Interactive outliner with full CRUD operations
- ✅ Cursor navigation (↑/↓)
- ✅ Expand/collapse (←/→)
- ✅ Edit mode (Enter)
- ✅ Node creation (Insert/`n`)
- ✅ Node deletion (Delete/`d`) with confirmation
- ✅ Node manipulation (Tab/Shift+Tab for indent/outdent)
- ✅ Real-time persistence

✅ **Phase 4 Complete**: Page management and navigation
- ✅ Multiple pages/notes support
- ✅ Page switcher (Ctrl+P)
- ✅ Page creation/deletion
- ✅ Sidebar pages list

✅ **Phase 5 Complete**: Search, tags, and backlinks
- ✅ Full-text search (/) with FTS5
- ✅ Tag system with filtering
- ✅ Wiki-style linking [[links]]
- ✅ Automatic backlinks
- ✅ Autocomplete for links and tags

✅ **Phase 6 Complete**: Calendar and daily notes
- ✅ Calendar widget in sidebar
- ✅ Date navigation and selection
- ✅ Daily notes creation
- ✅ Task management with priorities

✅ **Phase 7 Complete**: Attachments and transclusion
- ✅ File attachments with deduplication
- ✅ Image paste from clipboard (Ctrl+V)
- ✅ Transclusion syntax `![[Note Title#Node ID]]`
- ✅ Attachment management panel

✅ **Phase 8 Complete**: Polish and advanced features
- ✅ Mouse support for navigation
- ✅ Favorites system (Ctrl+F)
- ✅ Log book for task history (Ctrl+L)
- ✅ Export to Markdown (Ctrl+E)
- ✅ Page renaming (Ctrl+R)
- ✅ Task overview (Ctrl+Shift+T)

🎯 **MVP Complete**: All core features implemented and working

## Architecture

```
notiq/
├── core/           # Business logic library (UI-agnostic)
│   ├── models/     # Data structures
│   ├── storage/    # SQLite repositories
│   └── schema.sql  # Database schema
├── tui/            # Ratatui interface (Phase 2+)
├── cli/            # Command-line binary
└── tests/          # Integration tests
```

## Technology Stack

- **Language**: Rust 2021 Edition
- **Database**: SQLite with FTS5 for full-text search
- **TUI Framework**: Ratatui with Crossterm
- **File Handling**: Deduplication and hash-based storage
- **Search**: Full-text search with autocomplete
- **Export**: Markdown generation
- **Platform**: Cross-platform (Windows, macOS, Linux)

## Quick Start

```bash
# Clone and run
git clone <repository>
cd notiq
cargo run --bin notiq

# Or for development
cargo run --bin notiq --release

# Run tests
cargo test --workspace
```

## Key Features Working

### Core Outlining
- **Infinite nesting** with proper indentation
- **Expand/collapse** nodes (←/→)
- **Cursor navigation** (↑/↓)
- **Edit mode** (Enter to edit, Esc to cancel)
- **Node creation** (`n` or Insert)
- **Node deletion** (`d` or Delete with confirmation)
- **Indent/outdent** (Tab/Shift+Tab)

### Page Management
- **Multiple pages** with page switcher (Ctrl+P)
- **Page creation** (Ctrl+N)
- **Page deletion** (Ctrl+D)
- **Page renaming** (Ctrl+R)
- **Favorites** (Ctrl+F)

### Search & Navigation
- **Full-text search** (`/`)
- **Tag filtering** (#tag)
- **Wiki-style links** [[Page Title]]
- **Automatic backlinks**
- **Autocomplete** for links and tags

### Task Management
- **Task checkboxes** (`x` to toggle)
- **Task priorities** and due dates
- **Task overview** (Ctrl+Shift+T)
- **Task history** (Ctrl+L for logbook)

### Calendar & Daily Notes
- **Calendar widget** in sidebar
- **Date navigation** (Shift+Arrow keys)
- **Daily notes** (Shift+Enter)
- **Current day highlighting**

### Attachments & Files
- **File attachments** (Ctrl+A)
- **Image paste** from clipboard (Ctrl+V)
- **Attachment management** (Ctrl+O)
- **Transclusion** `![[Note Title#Node ID]]`

### Export & Data
- **Export to Markdown** (Ctrl+E)
- **Mouse support** for navigation
- **Sidebar toggle** (Ctrl+B)

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `q` | Quit application |
| `↑/↓` | Navigate outline |
| `←/→` | Expand/collapse nodes |
| `Enter` | Edit node |
| `Esc` | Cancel edit/close overlays |
| `n` | Create new node |
| `d` | Delete node (with confirmation) |
| `x` | Toggle task completion |
| `Tab/Shift+Tab` | Indent/outdent |
| `/` | Search |
| `Ctrl+P` | Page switcher |
| `Ctrl+N` | New page |
| `Ctrl+D` | Delete page |
| `Ctrl+R` | Rename page |
| `Ctrl+F` | Toggle favorite |
| `Ctrl+L` | Open logbook |
| `Ctrl+E` | Export to Markdown |
| `Ctrl+A` | Attach file |
| `Ctrl+V` | Paste image |
| `Ctrl+O` | Open attachments |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+Shift+T` | Task overview |
| `Shift+Arrow` | Calendar navigation |
| `Shift+Enter` | Open daily note |
| `[[/]]` | Navigate attachments |
| `Alt+↑/↓` | Reorder nodes |

## Current Status

The application is **feature-complete** with all planned MVP functionality implemented and working. Recent fixes include:

- ✅ Fixed AltGr key handling for special characters like `[` and `]`
- ✅ Fixed calendar date alignment
- ✅ Fixed double keypress issues on Windows
- ✅ Added page renaming functionality
- ✅ Improved mouse navigation support

## Future Enhancements (Phase 9)

Potential future additions based on user feedback:
- Tauri GUI layer for desktop
- Sync service for multi-device usage
- Mobile app (PWA or Tauri Mobile)
- Plugin system for extensibility
- Git integration for version control
- Advanced query language
- Graph view visualization
- Note templates
- Vim keybindings mode