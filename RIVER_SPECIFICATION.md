# River Text Editor - Complete Technical Specification

## Table of Contents
1. [Executive Summary](#executive-summary)
2. [System Overview](#system-overview)
3. [Technical Architecture](#technical-architecture)
4. [Module Specifications](#module-specifications)
5. [Data Structures](#data-structures)
6. [Core Algorithms](#core-algorithms)
7. [User Interface](#user-interface)
8. [Configuration System](#configuration-system)
9. [File Operations](#file-operations)
10. [Statistics and Tracking](#statistics-and-tracking)
11. [Build and Deployment](#build-and-deployment)
12. [External Dependencies](#external-dependencies)
13. [Platform-Specific Considerations](#platform-specific-considerations)
14. [Implementation Guidelines](#implementation-guidelines)

## Executive Summary

River is a minimalist terminal-based text editor designed specifically for daily journaling with a focus on achieving a 500-word daily writing goal. It provides real-time word count tracking, typing time monitoring, and automatic file management for daily notes.

### Key Features
- Terminal-based text editor with full UTF-8 support
- Real-time word count and progress tracking toward 500-word daily goal
- Automatic typing time tracking with configurable timeout
- Daily note management with automatic file creation
- Optional Vim keybindings
- Auto-save functionality
- Writing statistics dashboard
- Cross-platform support (macOS, Linux, Windows)

### Design Philosophy
- Minimalist interface to reduce distractions
- Focus on writing flow with automatic features
- Terminal-based for speed and simplicity
- Configurable but with sensible defaults

## System Overview

### Core Components
1. **Editor Engine**: Main text editing functionality with buffer management
2. **Terminal UI**: Crossterm-based terminal manipulation and rendering
3. **Configuration System**: TOML-based user preferences
4. **Statistics Tracker**: Daily typing time and word count persistence
5. **File Manager**: Automatic daily note creation and management

### Data Flow
1. **Startup**: Load config → Determine file to open → Load file → Enter raw mode
2. **Editing Loop**: Poll events → Process input → Update buffer → Render → Auto-save
3. **Statistics**: Track typing activity → Update session time → Persist to daily stats file
4. **Shutdown**: Save file → Save statistics → Leave raw mode

## Technical Architecture

### Application Structure
```
river/
├── src/
│   ├── main.rs         # Editor implementation and entry point
│   └── config.rs       # Configuration management
├── Cargo.toml          # Rust package manifest
├── Cargo.lock          # Dependency lock file
└── river.config.toml   # Example configuration
```

### Core Design Patterns
1. **State Machine**: Editor modes (Normal/Insert/Command)
2. **Event-Driven**: Keyboard event polling and handling
3. **Buffer Management**: Vec<Vec<char>> for line-based editing
4. **Immediate Rendering**: Dirty flag system for efficient updates

## Module Specifications

### Main Module (main.rs)

#### Editor Struct
```rust
struct Editor {
    // Text buffer - vector of lines, each line is vector of chars
    buffer: Vec<Vec<char>>,
    
    // Cursor position
    cursor_x: usize,
    cursor_y: usize,
    
    // Viewport scrolling offsets
    offset_x: usize,
    offset_y: usize,
    
    // Terminal dimensions
    terminal_width: u16,
    terminal_height: u16,
    
    // Rendering flag
    dirty: bool,
    
    // File management
    filename: Option<String>,
    needs_save: bool,
    last_save: Instant,
    
    // Editor mode
    mode: Mode,
    
    // Command line buffer
    command_buffer: String,
    
    // Copy/paste clipboard
    clipboard: Vec<Vec<char>>,
    
    // User configuration
    config: Config,
    
    // Typing session tracking
    typing_session_start: Option<Instant>,
    accumulated_typing_time: Duration,
    last_typing_activity: Instant,
}
```

#### Key Methods
- `new() -> io::Result<Self>`: Constructor, initializes editor state
- `run(&mut self) -> io::Result<()>`: Main event loop
- `handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<bool>`: Input dispatcher
- `render(&mut self) -> io::Result<()>`: Screen rendering
- `auto_save(&mut self) -> io::Result<()>`: Automatic file saving
- `count_words(&self) -> usize`: Word counting algorithm
- `track_typing(&mut self)`: Typing session management

### Configuration Module (config.rs)

#### Config Struct
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub vim_bindings: bool,              // Enable/disable vim mode
    pub tab_size: usize,                 // Spaces per tab
    pub daily_notes_dir: String,         // Directory for daily notes
    pub typing_timeout_seconds: u64,     // Typing session timeout
}
```

#### Configuration File Location
- **macOS**: `~/Library/Application Support/river/config.toml`
- **Linux**: `~/.config/river/config.toml`
- **Windows**: `%APPDATA%\river\config.toml`

## Data Structures

### Text Buffer
- **Type**: `Vec<Vec<char>>`
- **Description**: Line-based buffer where each line is a vector of UTF-8 characters
- **Rationale**: Efficient for line-based operations, natural for text editing

### Editor Modes
```rust
enum Mode {
    Normal,   // Vim normal mode (navigation)
    Insert,   // Text insertion mode
    Command,  // Command line mode
}
```

### Daily Statistics
```rust
#[derive(Debug, Serialize, Deserialize)]
struct DailyStats {
    typing_seconds: u64,    // Total typing time in seconds
    word_count: u64,        // Total words written
}
```

### Key Event Structure
- Uses Crossterm's `KeyEvent` with:
  - `code`: Key code (character, special key)
  - `modifiers`: Modifier keys (Ctrl, Alt, Shift)

## Core Algorithms

### Word Counting Algorithm
```
1. Initialize word_count = 0, in_word = false
2. For each line in buffer:
   3. For each character in line:
      4. If character is alphanumeric:
         5. If not in_word:
            6. Increment word_count
            7. Set in_word = true
      8. Else:
         9. Set in_word = false
   10. Set in_word = false (reset at line end)
11. Return word_count
```

### Typing Time Tracking
```
1. On any editing action:
   2. If no session active OR timeout exceeded:
      3. Start new session
   4. Update last_typing_activity timestamp
5. In main loop:
   6. If session active AND within timeout:
      7. Update accumulated time
   8. Else:
      9. End session
```

### Auto Line Wrapping
```
1. On character insertion:
   2. If cursor_x >= (terminal_width - 5):
      3. Find last space before cursor
      4. If space found and not too far back:
         5. Break at space
      6. Else:
         7. Break at current position
      8. Move remaining text to new line
      9. Update cursor position
```

### Viewport Scrolling
```
1. Calculate visible area (terminal_height - 2 lines for status)
2. If cursor above viewport:
   3. Scroll up to show cursor
4. If cursor below viewport:
   5. Scroll down to show cursor
6. Apply same logic for horizontal scrolling
```

## User Interface

### Layout
```
┌─────────────────────────────────────┐
│ Text content area                   │ <- Main editing area
│ ...                                 │
│ ...                                 │
│ ~                                   │ <- Empty line indicator
├─────────────────────────────────────┤
│ [████████░░] 250 words 50% · 5 min │ <- Status bar
│ :command                            │ <- Command line (when active)
└─────────────────────────────────────┘
```

### Status Bar Components
1. **Progress Bar**: Visual representation of 500-word goal
2. **Word Count**: Current document word count
3. **Percentage**: Progress percentage (capped at 100%)
4. **Typing Time**: Active typing time in minutes

### Color Scheme
- **Progress Bar Colors**:
  - Green: 100% or more (goal reached)
  - Yellow: 75-99%
  - White: Below 75%
- **Empty Lines**: Dark grey `~` character
- **Command Mode**: Shows command buffer on bottom line

### Key Bindings

#### Standard Mode (vim_bindings = false)
- **Navigation**: Arrow keys, Home, End, Page Up/Down
- **Editing**: Direct character input, Backspace, Delete, Enter, Tab
- **Control**: Ctrl+Q to quit

#### Vim Mode (vim_bindings = true)
##### Normal Mode
- **Movement**: h/j/k/l, 0/$, g/G, w/b/e
- **Mode Switch**: i/I/a/A (insert), o/O (new line), : (command)
- **Editing**: x (delete char), dd (delete line), yy (yank line), p/P (paste)

##### Insert Mode
- **Exit**: Esc (return to normal mode)
- **Navigation**: Arrow keys, Home, End, Page Up/Down
- **Editing**: Character input, Backspace, Delete, Enter, Tab

##### Command Mode
- **Commands**: :q (quit)
- **Exit**: Esc or Enter

## Configuration System

### Configuration File Format (TOML)
```toml
# Enable vim keybindings (true/false)
vim_bindings = false

# Tab size (number of spaces)
tab_size = 4

# Directory for daily notes (supports ~ expansion)
daily_notes_dir = "~/Documents/DailyNotes"

# Typing timeout in seconds
typing_timeout_seconds = 180
```

### Default Values
- `vim_bindings`: false
- `tab_size`: 4
- `daily_notes_dir`: ~/Documents/DailyNotes
- `typing_timeout_seconds`: 180 (3 minutes)

### Configuration Loading Process
1. Determine platform-specific config directory
2. Check if config.toml exists
3. If exists: Parse TOML, expand ~ to home directory
4. If not exists: Create default config file
5. Return configuration object

## File Operations

### Daily Note Management
1. **Filename Format**: `YYYY-MM-DD.md`
2. **Location**: Configured daily_notes_dir
3. **Creation**: Automatic on startup if not exists
4. **Header**: "# Day, Month DD, YYYY" format

### File Saving
1. **Auto-save Trigger**: 1 second after last edit
2. **Save Format**: UTF-8 text with LF line endings
3. **Atomic Write**: Not implemented (direct overwrite)

### Statistics Persistence
1. **Filename Format**: `.stats-YYYY-MM-DD.toml`
2. **Location**: Same as daily_notes_dir
3. **Update Frequency**: Every 10 seconds during active typing
4. **Format**: TOML with typing_seconds and word_count fields

## Statistics and Tracking

### Statistics Display (--stats flag)
Shows a dashboard with:
1. **Today's Stats**: Current day typing time
2. **Current Streak**: Consecutive days with typing activity
3. **Weekly Average**: Average typing time over last 7 days
4. **Total Notes**: Count of daily note files
5. **7-Day Chart**: Bar chart showing typing time and word count

### Data Collection
- **Typing Time**: Accumulated during active typing sessions
- **Word Count**: Real-time calculation from buffer
- **Session Definition**: Continuous typing with gaps < timeout
- **Persistence**: Daily TOML files with cumulative stats

## Build and Deployment

### Dependencies
```toml
[dependencies]
crossterm = "0.28"      # Terminal manipulation
serde = { version = "1.0", features = ["derive"] }  # Serialization
toml = "0.8"           # TOML parsing
dirs = "5.0"           # Platform directories
chrono = "0.4"         # Date/time handling
```

### Build Process
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Install system-wide
sudo cp ./target/release/river /usr/local/bin/
```

### Binary Details
- **Name**: river
- **Type**: Single static binary
- **Size**: ~2-4 MB (release build)
- **Dependencies**: None at runtime

## External Dependencies

### Crossterm (v0.28)
- **Purpose**: Terminal manipulation
- **Features Used**:
  - Raw mode for direct input
  - Alternate screen buffer
  - Cursor positioning
  - Color output
  - Event polling

### Serde (v1.0)
- **Purpose**: Serialization framework
- **Features**: derive (automatic implementation)
- **Usage**: Config and stats serialization

### TOML (v0.8)
- **Purpose**: Configuration file format
- **Usage**: Parse and generate TOML files

### Dirs (v5.0)
- **Purpose**: Platform-specific directories
- **Usage**: Config and home directory paths

### Chrono (v0.4)
- **Purpose**: Date and time handling
- **Usage**: Daily note filenames, statistics

## Platform-Specific Considerations

### macOS
- **Config Path**: ~/Library/Application Support/river/
- **Terminal**: Typically Terminal.app or iTerm2
- **Special Keys**: Command key not used

### Linux
- **Config Path**: ~/.config/river/
- **Terminal**: Various (xterm, gnome-terminal, etc.)
- **Permissions**: Standard Unix permissions

### Windows
- **Config Path**: %APPDATA%\river\
- **Terminal**: Windows Terminal, ConEmu, cmd.exe
- **Line Endings**: Converts to LF internally

### Terminal Requirements
- **Minimum Size**: 80x24 recommended
- **Encoding**: UTF-8
- **Features**: ANSI escape sequences, 256 colors

## Implementation Guidelines

### Error Handling
- Use `Result<T, E>` for all fallible operations
- Propagate errors with `?` operator
- Display user-friendly error messages
- Gracefully handle terminal resize

### Performance Considerations
- Render only when dirty flag is set
- Use efficient data structures (Vec for lines)
- Minimize allocations in hot paths
- Batch terminal updates

### Code Organization
- Keep modules focused and cohesive
- Use clear, descriptive names
- Document complex algorithms
- Follow Rust idioms and conventions

### Testing Strategy
- Unit tests for core algorithms
- Integration tests for file operations
- Manual testing for terminal UI
- Cross-platform verification

### Future Extensibility
- Modular design for easy feature addition
- Configuration-driven behavior
- Clean separation of concerns
- Well-defined internal APIs

### Security Considerations
- No network operations
- Local file access only
- No arbitrary code execution
- Safe handling of user input

This specification provides a complete blueprint for reimplementing River in any programming language while maintaining feature parity and design philosophy.