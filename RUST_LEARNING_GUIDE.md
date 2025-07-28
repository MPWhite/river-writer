# Rust Learning Guide for River

This guide explains the key Rust concepts used in River with practical examples.

## 1. Ownership and Borrowing

Rust's ownership system is its most distinctive feature. Every value has a single owner, and when the owner goes out of scope, the value is dropped.

### Example from River:
```rust
// In save_file method
if let Some(filename) = &self.filename {
    // &self.filename borrows the filename instead of taking ownership
    // This allows us to use self.filename later
}
```

### Key Rules:
- Each value has a single owner
- When owner goes out of scope, value is dropped
- You can have multiple immutable borrows OR one mutable borrow

## 2. Pattern Matching

Pattern matching in Rust is exhaustive - you must handle all possible cases.

### Example from River:
```rust
match self.mode {
    Mode::Normal => self.handle_normal_mode(key_event),
    Mode::Insert => self.handle_vim_insert_mode(key_event),
    Mode::Command => self.handle_command_mode(key_event),
}
```

### Match Guards:
```rust
KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
    // Only matches Char('q') when Ctrl is also pressed
}
```

## 3. Option Type

Option<T> represents a value that might be present (Some) or absent (None). It's Rust's null-safety mechanism.

### Examples from River:
```rust
// Option in struct field
filename: Option<String>,  // File might not have a name yet

// Pattern matching on Option
if let Some(filename) = &self.filename {
    // filename is available here
}

// Using unwrap_or_default()
let day_str = some_option.unwrap_or_default(); // Returns empty string if None
```

## 4. Result Type and Error Handling

Result<T, E> represents operations that might fail. The ? operator propagates errors.

### Examples from River:
```rust
// Function returning Result
fn new() -> io::Result<Self> {
    let (width, height) = terminal::size()?; // ? propagates error if size() fails
    // ...
    Ok(editor) // Wrap success value in Ok
}

// Handling errors explicitly
if let Err(e) = self.save() {
    eprintln!("Error: {}", e);
}
```

## 5. Iterators

Iterators provide a functional programming style for working with collections.

### Example from River:
```rust
// Converting buffer to string
let content: String = self.buffer
    .iter()                                    // Create iterator
    .map(|line| line.iter().collect::<String>()) // Transform each element
    .collect::<Vec<String>>()                  // Collect into Vec
    .join("\n");                               // Join with newlines
```

### Common Iterator Methods:
- `iter()` - iterate over references
- `map()` - transform each element
- `filter()` - keep only matching elements
- `take(n)` - take first n elements
- `collect()` - consume iterator into collection

## 6. Lifetimes

While River doesn't explicitly use lifetime annotations, they're implicit in many places.

### Implicit Lifetimes:
```rust
fn current_line(&self) -> &Vec<char> {
    // Rust infers: the returned reference lives as long as &self
    &self.buffer[self.cursor_y]
}
```

## 7. Traits

Traits define shared behavior. River uses several standard traits:

### Derive Macros:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode { /* ... */ }
```

This automatically implements:
- `Debug` - for {:?} formatting
- `Clone` - for .clone()
- `Copy` - for implicit copying
- `PartialEq` - for == comparison

### Serde Traits:
```rust
#[derive(Serialize, Deserialize)]
struct Config { /* ... */ }
```

Enables automatic serialization to/from formats like TOML.

## 8. Closures

Closures are anonymous functions that can capture their environment.

### Examples from River:
```rust
// Simple closure in map
.map(|line| line.iter().collect::<String>())

// Closure with error handling
.unwrap_or_else(|e| {
    eprintln!("Error: {}", e);
    Self::default()
})
```

## 9. String Types

Rust has two main string types:
- `String` - owned, heap-allocated, mutable
- `&str` - borrowed string slice

### Examples:
```rust
let owned: String = "hello".to_string();  // Creates owned String
let slice: &str = &owned;                 // Borrows as &str
let literal: &str = "world";              // String literals are &str
```

## 10. Common Patterns in River

### Builder Pattern with Defaults:
```rust
impl Default for Config {
    fn default() -> Self {
        Config {
            vim_bindings: false,
            tab_size: 4,
            // ...
        }
    }
}
```

### Type State Pattern:
Different behavior based on current state (Mode enum).

### RAII (Resource Acquisition Is Initialization):
```rust
// Raw mode is enabled in enter_raw_mode()
// and automatically disabled in leave_raw_mode()
// when Editor goes out of scope
```

## 11. Macros

Macros generate code at compile time.

### Examples from River:
```rust
// format! - string interpolation
let status = format!("Words: {}", count);

// println!/eprintln! - printing
eprintln!("Error: {}", e);  // Prints to stderr

// vec! - vector creation
let v = vec![1, 2, 3];
```

## 12. Module System

Rust's module system organizes code:

```rust
mod config;          // Declares a module
use config::Config;  // Brings Config into scope
pub struct Editor    // pub makes it public
```

## Tips for Learning Rust

1. **Ownership Mindset**: Think about who owns data and for how long
2. **Embrace the Compiler**: Rust's error messages are very helpful
3. **Use Pattern Matching**: It's more powerful than if/else chains
4. **Iterators Over Loops**: When possible, use iterator methods
5. **Handle Errors Explicitly**: Don't unwrap() in production code

## Common Gotchas

1. **Moving vs Borrowing**: Use & when you want to keep using a value
2. **Mutable References**: Can only have one at a time
3. **String Conversions**: Know when to use .to_string(), .to_owned(), or .into()
4. **Index Bounds**: Array indexing can panic - use .get() for safe access

## Resources for Further Learning

- The Rust Book: https://doc.rust-lang.org/book/
- Rust by Example: https://doc.rust-lang.org/rust-by-example/
- Rustlings (exercises): https://github.com/rust-lang/rustlings
- Rust Playground: https://play.rust-lang.org/