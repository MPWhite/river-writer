# River

A minimalist daily journal editor that encourages you to write 500 words every day.

## Features

- **Daily Notes**: Automatically creates and opens today's journal entry
- **Writing Goal**: Visual progress bar tracking your daily 500-word target
- **Typing Time Tracker**: Measures actual writing time (not just app open time)
- **Auto-save**: Never lose your work - saves automatically as you type
- **Vim Mode**: Optional vim keybindings for power users
- **Clean UI**: Minimal, distraction-free interface focused on writing

## Installation

### Prerequisites

- Rust (install from https://rustup.rs/)

### Build from source

```bash
git clone https://github.com/yourusername/river.git
cd river
cargo build --release
```

The binary will be at `./target/release/river`

### Install globally

```bash
sudo cp ./target/release/river /usr/local/bin/
```

## Usage

### Basic Usage

Simply run `river` to open today's daily note:

```bash
river
```

Each day gets its own file named `YYYY-MM-DD.md` (e.g., `2024-12-19.md`).

### Open a specific file

```bash
river myfile.txt
```

### Key Bindings

**Standard Mode** (default):
- Type normally - all standard keys work as expected
- `Ctrl+Q` - Quit
- Arrow keys, Home, End, Page Up/Down - Navigation
- Auto-saves every second after you stop typing

**Vim Mode** (when enabled):
- Normal mode: `h/j/k/l` movement, `i` insert, `dd` delete line, etc.
- Insert mode: `Esc` to return to normal mode
- Command mode: `:q` quit, `/search` to find text

## Configuration

River looks for configuration at:
- macOS: `~/Library/Application Support/river/config.toml`
- Linux: `~/.config/river/config.toml`

### Example config.toml

```toml
# Enable vim keybindings (true/false)
vim_bindings = false

# Tab size (number of spaces)
tab_size = 4

# Directory where daily notes are stored
daily_notes_dir = "~/Documents/DailyNotes"
```

## Status Bar

The minimal status bar shows your writing progress:

```
[=================                    ] 250 words  50% · 12 min
```

- **Progress bar**: Visual representation of your 500-word goal
- **Word count**: Current document word count
- **Percentage**: Progress toward daily goal
- **Time**: Minutes spent actively typing today

The status bar changes color:
- White: Under 375 words
- Yellow: 375-499 words (getting close!)
- Green: 500+ words (goal achieved!)

## Features in Detail

### Daily Notes
- Each day automatically creates a new file
- Files include a formatted date header
- All notes stored in your configured directory

### Typing Time Tracking
- Only counts time when actively typing
- 5-second timeout - stops counting if you pause
- Persists across sessions for the day
- Resets at midnight

### Auto-save
- Saves 1 second after you stop typing
- No manual save needed
- "[Modified]" indicator removed to reduce clutter

### Line Wrapping
- Automatic word wrap at terminal edge
- Smart breaking at word boundaries
- No horizontal scrolling needed

## Philosophy

River is designed around a simple idea: write 500 words every day. The minimal interface removes distractions and focuses on two things - your words and your progress. 

No plugins, no themes, no configuration overload. Just you and your daily writing practice.

## License

MIT