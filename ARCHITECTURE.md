# River Architecture & Rust Learning Guide

## Overview

River is a minimalist text editor written in Rust that tracks your daily 500-word writing goal. This document will help you understand both the architecture and the Rust concepts used throughout the project.

## Project Structure

```
river/
├── src/
│   ├── main.rs     # Main application logic and editor implementation
│   └── config.rs   # Configuration management module
├── Cargo.toml      # Rust project manifest (like package.json in Node.js)
└── Cargo.lock      # Locked dependencies (like package-lock.json)
```

## Key Rust Concepts Used

### 1. **Ownership and Borrowing**
Rust's most unique feature - memory safety without garbage collection.
- Every value has a single owner
- Values can be borrowed (referenced) immutably or mutably
- When owner goes out of scope, value is automatically dropped

### 2. **Structs and Implementations**
- `struct Editor` - Main editor state (like a class in other languages)
- `impl Editor` - Methods for the Editor struct
- `#[derive(...)]` - Automatic trait implementations

### 3. **Enums and Pattern Matching**
- `enum Mode` - Represents editor modes (Normal/Insert/Command)
- `match` expressions - Exhaustive pattern matching

### 4. **Error Handling**
- `Result<T, E>` - Type for operations that can fail
- `?` operator - Propagate errors up the call stack
- `io::Result<T>` - Shorthand for `Result<T, io::Error>`

### 5. **Modules and Visibility**
- `mod config` - Declares a module
- `pub` - Makes items public
- `use` - Brings items into scope

## Core Components

### 1. **Terminal UI (Crossterm)**
River uses the `crossterm` crate for terminal manipulation:
- Raw mode - Direct keyboard input without line buffering
- Alternate screen - Separate screen buffer (like vim)
- ANSI escape sequences - For colors and cursor movement

### 2. **Editor State (`struct Editor`)**
Main fields:
- `buffer: Vec<Vec<char>>` - Text storage (vector of lines, each line is vector of chars)
- `cursor_x/y` - Current cursor position
- `mode` - Current editor mode (vim-like)
- `config` - User configuration
- `typing_session_start` - Tracks typing time

### 3. **Event Loop**
The `run()` method contains the main event loop:
1. Render the current state
2. Poll for keyboard events
3. Handle the event based on current mode
4. Auto-save if needed
5. Update typing statistics

### 4. **Configuration System**
- Uses TOML format for configuration
- Serde for serialization/deserialization
- Platform-specific config paths

## Key Rust Patterns in This Project

### 1. **Builder Pattern with Default**
```rust
#[derive(Default)]
struct Config { ... }
```

### 2. **Type State Pattern**
Different behavior based on `Mode` enum state

### 3. **Iterator Chains**
```rust
self.buffer.iter()
    .map(|line| line.iter().collect::<String>())
    .collect::<Vec<String>>()
    .join("\n")
```

### 4. **Lifetime Elision**
Most lifetimes are inferred by the compiler

### 5. **Trait Bounds**
Generic constraints like `T: Display` (not heavily used here)

## Data Flow

1. **Startup**: Load config → Open/create daily note → Enter raw mode
2. **Editing**: Key event → Mode-specific handler → Update buffer → Mark dirty
3. **Rendering**: Calculate viewport → Draw lines → Draw status bar → Position cursor
4. **Auto-save**: Check timer → Write buffer to file → Update save timestamp
5. **Shutdown**: Save file → Save typing stats → Leave raw mode

## Memory Management

Rust automatically manages memory through:
- Stack allocation for fixed-size values
- Heap allocation for dynamic data (Vec, String)
- RAII (Resource Acquisition Is Initialization)
- No manual memory management needed!

## Concurrency Considerations

This project is single-threaded, but Rust's ownership system would ensure thread safety if we added concurrency through:
- `Send` trait - Safe to transfer between threads
- `Sync` trait - Safe to share references between threads

## Testing Considerations

While not implemented here, Rust testing typically includes:
- Unit tests with `#[test]`
- Integration tests in `tests/` directory
- Documentation tests in doc comments