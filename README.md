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
typing_timeout_seconds = 180  # 3 minutes
```

## TODO

Here is a random list of things I think might be cool to add, in no particular order: 

* Stats page - Basic stats like words per day, etc. More sophisticated stats driven by LLM like mood over time, various emotions over itme, etc. etc. Not sure how valuable that would be, but I think it would be neat
* Chat - Chat with your notes, idk... why not? 
* Default prompts - ideally driven by prior insights 
* Sync??
