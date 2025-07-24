# River

A minimal text editor that tracks your daily 500-word writing goal.

## Install

```bash
cargo build --release
sudo cp ./target/release/river /usr/local/bin/
```

## Usage

```bash
river              # Opens today's journal
river file.txt     # Opens specific file
```

**Controls**: Just type. `Ctrl+Q` to quit. Auto-saves.

**Status bar**: Shows words, progress bar, and typing time.

## Config

`~/Library/Application Support/river/config.toml` (macOS)

```toml
vim_bindings = false
daily_notes_dir = "~/Documents/DailyNotes"
```