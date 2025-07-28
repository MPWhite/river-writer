// External crate imports - these are declared in Cargo.toml
// 'use' brings items into scope, similar to 'import' in other languages
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{
        self, Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
// Standard library imports
// 'std' is Rust's standard library, always available
// 'self' in imports refers to the module itself (for functions)
use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf}; // Path manipulation types
use std::fs; // File system operations
use chrono::Local; // External crate for date/time handling
use serde::{Deserialize, Serialize}; // Serialization traits

// Module declaration - tells Rust to look for config.rs or config/mod.rs
mod config;
// Bring Config struct into scope from our config module
use config::Config;

// Enums in Rust are algebraic data types - they can only be one variant at a time
// #[derive(...)] automatically implements common traits:
// - Debug: allows {:?} formatting
// - Clone: allows .clone() to create copies
// - Copy: allows implicit copying (for small, stack-allocated types)
// - PartialEq: allows == comparison
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Normal,  // Vim normal mode
    Insert,  // Text insertion mode
    Command, // Command line mode (for :commands and /search)
}

// Structs are like classes in other languages, but without inheritance
// Serialize/Deserialize traits enable conversion to/from formats like JSON/TOML
#[derive(Debug, Serialize, Deserialize)]
struct DailyStats {
    // #[serde(default)] uses Default::default() if field is missing during deserialization
    #[serde(default)]
    typing_seconds: u64, // u64 is an unsigned 64-bit integer
    // Future stats can be added here
}

// 'impl' blocks add methods to types
// Default trait provides a default value for a type
impl Default for DailyStats {
    // 'Self' is an alias for the type we're implementing on (DailyStats)
    fn default() -> Self {
        DailyStats {
            typing_seconds: 0,
        }
    }
}

// Main editor struct - holds all state for the text editor
struct Editor {
    // Vec<T> is a growable array (like ArrayList in Java or vector in C++)
    // Vec<Vec<char>> represents lines of text, where each line is a vector of characters
    buffer: Vec<Vec<char>>,
    
    // usize is the pointer-sized unsigned integer type (32/64 bit depending on architecture)
    cursor_x: usize,          // Current cursor column
    cursor_y: usize,          // Current cursor line
    offset_y: usize,          // Viewport vertical scroll offset
    offset_x: usize,          // Viewport horizontal scroll offset
    
    // u16 is unsigned 16-bit integer
    terminal_height: u16,
    terminal_width: u16,
    
    dirty: bool,              // Whether screen needs redrawing
    
    // Option<T> represents an optional value - either Some(T) or None
    // This is Rust's null-safety mechanism
    filename: Option<String>,
    
    mode: Mode,               // Current editor mode (enum defined above)
    
    // String is a heap-allocated, growable UTF-8 string
    // (different from &str which is a string slice/reference)
    command_buffer: String,
    
    clipboard: Vec<Vec<char>>, // For copy/paste operations
    last_search: Option<String>,
    config: Config,           // User configuration
    needs_save: bool,
    
    // Instant represents a point in time for measuring durations
    last_save: Instant,
    typing_session_start: Option<Instant>,
    
    // Duration represents a span of time
    accumulated_typing_time: Duration,
    last_typing_activity: Instant,
}

// Implementation block for Editor methods
impl Editor {
    // Constructor function - by convention named 'new'
    // Returns io::Result<Self> which is Result<Self, io::Error>
    // Result<T, E> is Rust's error handling type - either Ok(T) or Err(E)
    fn new() -> io::Result<Self> {
        // ? operator propagates errors - if terminal::size() returns Err, 
        // this function immediately returns that error
        let (width, height) = terminal::size()?;
        
        // Load configuration from file
        let config = Config::load();
        
        // Conditional expression - like ternary operator but more readable
        let mode = if config.vim_bindings {
            Mode::Normal
        } else {
            Mode::Insert
        };
        
        // Self:: refers to the type itself (for associated functions)
        // &config passes a reference (borrow) instead of moving ownership
        let accumulated_time = Self::load_typing_time(&config)?;
        
        // Ok() wraps the value in Result::Ok variant
        Ok(Editor {
            buffer: vec![Vec::new()],
            cursor_x: 0,
            cursor_y: 0,
            offset_y: 0,
            offset_x: 0,
            terminal_height: height,
            terminal_width: width,
            dirty: false,
            filename: None,
            mode,
            command_buffer: String::new(),
            clipboard: Vec::new(),
            last_search: None,
            config,
            needs_save: false,
            last_save: Instant::now(),
            typing_session_start: None,
            accumulated_typing_time: accumulated_time,
            last_typing_activity: Instant::now(),
        })
    }

    // Main event loop method
    // &mut self - mutable borrow of self (can modify the struct)
    // () is the unit type - like void in other languages
    fn run(&mut self) -> io::Result<()> {
        self.enter_raw_mode()?;
        
        let mut last_typing_save = Instant::now();
        
        // 'loop' creates an infinite loop (like while(true))
        loop {
            self.render()?;
            
            // Auto-save logic: save after 1 second of inactivity
            // && is logical AND, short-circuits if first condition is false
            if self.needs_save && self.last_save.elapsed() > Duration::from_secs(1) {
                self.auto_save()?;
            }
            
            // Update accumulated typing time if actively typing
            // 'if let' is pattern matching - only runs if pattern matches
            // Extracts the value from Some(session_start), skips if None
            if let Some(session_start) = self.typing_session_start {
                let typing_timeout = Duration::from_secs(self.config.typing_timeout_seconds);
                if self.last_typing_activity.elapsed() <= typing_timeout {
                    self.accumulated_typing_time = self.accumulated_typing_time + 
                        self.last_typing_activity.duration_since(session_start);
                    self.typing_session_start = Some(self.last_typing_activity);
                } else {
                    // Session ended, clear it
                    self.typing_session_start = None;
                }
            }
            
            // Save typing time every 10 seconds
            if last_typing_save.elapsed() > Duration::from_secs(10) {
                let _ = self.save_typing_time();
                last_typing_save = Instant::now();
            }
            
            // Poll for events with 16ms timeout (roughly 60 FPS)
            if event::poll(Duration::from_millis(16))? {
                // Pattern match on event type
                if let Event::Key(key_event) = event::read()? {
                    // If handle_key_event returns true, exit the loop
                    if self.handle_key_event(key_event)? {
                        break; // 'break' exits the innermost loop
                    }
                }
            }
            
            if let Ok((width, height)) = terminal::size() {
                if width != self.terminal_width || height != self.terminal_height {
                    self.terminal_width = width;
                    self.terminal_height = height;
                    self.dirty = true;
                }
            }
        }
        
        // Save before exiting
        if self.needs_save {
            self.auto_save()?;
        }
        let _ = self.save_typing_time();
        
        self.leave_raw_mode()?;
        Ok(())
    }

    fn enter_raw_mode(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            DisableLineWrap,
            Hide,
            Clear(ClearType::All)
        )?;
        self.dirty = true;
        Ok(())
    }

    fn leave_raw_mode(&mut self) -> io::Result<()> {
        execute!(
            io::stdout(),
            Show,
            EnableLineWrap,
            LeaveAlternateScreen
        )?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    // Dispatch key events based on current mode
    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<bool> {
        if self.config.vim_bindings {
            // 'match' is exhaustive pattern matching - must handle all variants
            // Similar to switch/case but more powerful
            match self.mode {
                Mode::Normal => self.handle_normal_mode(key_event),
                Mode::Insert => self.handle_vim_insert_mode(key_event),
                Mode::Command => self.handle_command_mode(key_event),
            }
        } else {
            self.handle_standard_mode(key_event)
        }
    }

    fn handle_standard_mode(&mut self, key_event: KeyEvent) -> io::Result<bool> {
        // Pattern matching on enum variants with destructuring
        // KeyCode is an enum with many variants (Char, Enter, etc.)
        match key_event.code {
            // Match guards: 'if' after pattern adds extra condition
            // KeyModifiers is a bitflag, contains() checks if flag is set
            KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Home => self.move_home(),
            KeyCode::End => self.move_end(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            KeyCode::Enter => self.insert_newline(),
            KeyCode::Tab => self.insert_tab(),
            // Pattern binding: 'c' captures the character inside Char variant
            KeyCode::Char(c) => {
                // Bitwise OR combines flags, intersects() checks if ANY are set
                // ! is logical NOT
                if !key_event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
                    self.insert_char(c);
                }
            }
            // _ is wildcard pattern - matches anything not handled above
            _ => {}
        }
        Ok(false)
    }

    fn handle_normal_mode(&mut self, key_event: KeyEvent) -> io::Result<bool> {
        match key_event.code {
            KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.command_buffer.clear();
                self.dirty = true;
            }
            KeyCode::Char('i') => {
                self.mode = Mode::Insert;
                self.dirty = true;
            }
            KeyCode::Char('I') => {
                self.move_home();
                self.mode = Mode::Insert;
                self.dirty = true;
            }
            KeyCode::Char('a') => {
                if self.cursor_x < self.current_line().len() {
                    self.cursor_x += 1;
                }
                self.mode = Mode::Insert;
                self.dirty = true;
            }
            KeyCode::Char('A') => {
                self.move_end();
                self.mode = Mode::Insert;
                self.dirty = true;
            }
            KeyCode::Char('o') => {
                self.move_end();
                self.insert_newline();
                self.mode = Mode::Insert;
                self.dirty = true;
            }
            KeyCode::Char('O') => {
                self.move_home();
                self.buffer.insert(self.cursor_y, Vec::new());
                self.dirty = true;
                self.needs_save = true;
                self.last_save = Instant::now();
                self.mode = Mode::Insert;
            }
            KeyCode::Char('h') | KeyCode::Left => self.move_left(),
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('l') | KeyCode::Right => self.move_right(),
            KeyCode::Char('0') | KeyCode::Home => self.move_home(),
            KeyCode::Char('$') | KeyCode::End => self.move_end(),
            KeyCode::Char('g') => {
                self.cursor_y = 0;
                self.cursor_x = 0;
                self.dirty = true;
            }
            KeyCode::Char('G') => {
                self.cursor_y = self.buffer.len() - 1;
                self.cursor_x = 0;
                self.dirty = true;
            }
            KeyCode::Char('w') => self.move_word_forward(),
            KeyCode::Char('b') => self.move_word_backward(),
            KeyCode::Char('e') => self.move_word_end(),
            KeyCode::Char('x') => self.delete_char(),
            KeyCode::Char('d') => {
                if self.last_key_was('d') {
                    self.delete_line();
                }
            }
            KeyCode::Char('y') => {
                if self.last_key_was('y') {
                    self.yank_line();
                }
            }
            KeyCode::Char('p') => self.paste_after(),
            KeyCode::Char('P') => self.paste_before(),
            KeyCode::Char('/') => {
                self.mode = Mode::Command;
                self.command_buffer = "/".to_string();
                self.dirty = true;
            }
            KeyCode::Char('n') => self.search_next(),
            KeyCode::Char('N') => self.search_prev(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            _ => {}
        }
        Ok(false)
    }

    fn handle_vim_insert_mode(&mut self, key_event: KeyEvent) -> io::Result<bool> {
        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                if self.cursor_x > 0 && self.cursor_x == self.current_line().len() {
                    self.cursor_x -= 1;
                }
                self.dirty = true;
            }
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Home => self.move_home(),
            KeyCode::End => self.move_end(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            KeyCode::Enter => self.insert_newline(),
            KeyCode::Tab => self.insert_tab(),
            KeyCode::Char(c) => {
                if !key_event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
                    self.insert_char(c);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_command_mode(&mut self, key_event: KeyEvent) -> io::Result<bool> {
        match key_event.code {
            KeyCode::Esc => {
                if self.config.vim_bindings {
                    self.mode = Mode::Normal;
                } else {
                    self.mode = Mode::Insert;
                }
                self.command_buffer.clear();
                self.dirty = true;
            }
            KeyCode::Enter => {
                let result = self.execute_command();
                if self.config.vim_bindings {
                    self.mode = Mode::Normal;
                } else {
                    self.mode = Mode::Insert;
                }
                self.command_buffer.clear();
                self.dirty = true;
                return result;
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
                if self.command_buffer.is_empty() {
                    if self.config.vim_bindings {
                        self.mode = Mode::Normal;
                    } else {
                        self.mode = Mode::Insert;
                    }
                }
                self.dirty = true;
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
                self.dirty = true;
            }
            _ => {}
        }
        Ok(false)
    }

    fn execute_command(&mut self) -> io::Result<bool> {
        if self.command_buffer.starts_with('/') {
            let search_term = self.command_buffer[1..].to_string();
            if !search_term.is_empty() {
                self.last_search = Some(search_term);
                self.search_next();
            }
        } else if self.config.vim_bindings {
            match self.command_buffer.as_str() {
                "q" => return Ok(true),
                _ => {}
            }
        }
        Ok(false)
    }

    fn last_key_was(&self, _c: char) -> bool {
        // Simplified for now - in a real implementation, we'd track the last key
        true
    }

    // Movement methods - note they take &mut self to modify cursor position
    fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1; // -= is compound assignment
        } else if self.cursor_y > 0 && (self.mode == Mode::Insert || !self.config.vim_bindings) {
            self.cursor_y -= 1;
            // Method calls use . notation
            self.cursor_x = self.current_line().len();
        }
        self.dirty = true;
    }

    fn move_right(&mut self) {
        let line_len = self.current_line().len();
        let max_x = if self.mode == Mode::Normal && line_len > 0 && self.config.vim_bindings {
            line_len - 1
        } else {
            line_len
        };
        
        if self.cursor_x < max_x {
            self.cursor_x += 1;
        } else if self.cursor_y < self.buffer.len() - 1 && (self.mode == Mode::Insert || !self.config.vim_bindings) {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
        self.dirty = true;
    }

    fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            let line_len = self.current_line().len();
            let max_x = if self.mode == Mode::Normal && line_len > 0 && self.config.vim_bindings {
                line_len - 1
            } else {
                line_len
            };
            self.cursor_x = self.cursor_x.min(max_x);
            self.dirty = true;
        }
    }

    fn move_down(&mut self) {
        if self.cursor_y < self.buffer.len() - 1 {
            self.cursor_y += 1;
            let line_len = self.current_line().len();
            let max_x = if self.mode == Mode::Normal && line_len > 0 && self.config.vim_bindings {
                line_len - 1
            } else {
                line_len
            };
            self.cursor_x = self.cursor_x.min(max_x);
            self.dirty = true;
        }
    }

    fn move_home(&mut self) {
        self.cursor_x = 0;
        self.dirty = true;
    }

    fn move_end(&mut self) {
        let line_len = self.current_line().len();
        self.cursor_x = if self.mode == Mode::Normal && line_len > 0 && self.config.vim_bindings {
            line_len - 1
        } else {
            line_len
        };
        self.dirty = true;
    }

    fn move_word_forward(&mut self) {
        let line = self.current_line();
        let mut x = self.cursor_x;
        
        // Skip current word
        while x < line.len() && line[x].is_alphanumeric() {
            x += 1;
        }
        // Skip spaces
        while x < line.len() && !line[x].is_alphanumeric() {
            x += 1;
        }
        
        if x < line.len() {
            self.cursor_x = x;
        } else if self.cursor_y < self.buffer.len() - 1 {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
        self.dirty = true;
    }

    fn move_word_backward(&mut self) {
        if self.cursor_x == 0 {
            if self.cursor_y > 0 {
                self.cursor_y -= 1;
                self.cursor_x = self.current_line().len();
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
            }
            return;
        }
        
        let line = self.current_line();
        let mut x = self.cursor_x - 1;
        
        // Skip spaces
        while x > 0 && !line[x].is_alphanumeric() {
            x -= 1;
        }
        // Skip word
        while x > 0 && line[x - 1].is_alphanumeric() {
            x -= 1;
        }
        
        self.cursor_x = x;
        self.dirty = true;
    }

    fn move_word_end(&mut self) {
        let line = self.current_line();
        let mut x = self.cursor_x;
        
        if x < line.len() - 1 {
            x += 1;
            // Skip to end of current word
            while x < line.len() - 1 && line[x].is_alphanumeric() {
                x += 1;
            }
            self.cursor_x = x;
        } else if self.cursor_y < self.buffer.len() - 1 {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
        self.dirty = true;
    }

    fn delete_char(&mut self) {
        self.track_typing(); // Track typing activity
        
        if self.cursor_x < self.current_line().len() {
            self.buffer[self.cursor_y].remove(self.cursor_x);
            if self.cursor_x > 0 && self.cursor_x == self.current_line().len() && self.config.vim_bindings {
                self.cursor_x -= 1;
            }
            self.dirty = true;
            self.needs_save = true;
            self.last_save = Instant::now();
        }
    }

    fn delete_line(&mut self) {
        self.track_typing(); // Track typing activity
        
        self.clipboard = vec![self.buffer[self.cursor_y].clone()];
        if self.buffer.len() > 1 {
            self.buffer.remove(self.cursor_y);
            if self.cursor_y >= self.buffer.len() {
                self.cursor_y = self.buffer.len() - 1;
            }
        } else {
            self.buffer[0].clear();
        }
        self.cursor_x = 0;
        self.dirty = true;
        self.needs_save = true;
        self.last_save = Instant::now();
    }

    fn yank_line(&mut self) {
        self.clipboard = vec![self.buffer[self.cursor_y].clone()];
    }

    fn paste_after(&mut self) {
        if !self.clipboard.is_empty() {
            self.track_typing(); // Track typing activity
            
            for (i, line) in self.clipboard.iter().enumerate() {
                self.buffer.insert(self.cursor_y + 1 + i, line.clone());
            }
            self.cursor_y += 1;
            self.cursor_x = 0;
            self.dirty = true;
            self.needs_save = true;
            self.last_save = Instant::now();
        }
    }

    fn paste_before(&mut self) {
        if !self.clipboard.is_empty() {
            self.track_typing(); // Track typing activity
            
            for (i, line) in self.clipboard.iter().enumerate() {
                self.buffer.insert(self.cursor_y + i, line.clone());
            }
            self.cursor_x = 0;
            self.dirty = true;
            self.needs_save = true;
            self.last_save = Instant::now();
        }
    }

    fn search_next(&mut self) {
        if let Some(search) = &self.last_search {
            let search_chars: Vec<char> = search.chars().collect();
            let mut found = false;
            
            // Search from current position
            for y in self.cursor_y..self.buffer.len() {
                let start_x = if y == self.cursor_y { self.cursor_x + 1 } else { 0 };
                let line = &self.buffer[y];
                
                for x in start_x..line.len() {
                    if x + search_chars.len() <= line.len() {
                        let matches = (0..search_chars.len())
                            .all(|i| line[x + i] == search_chars[i]);
                        if matches {
                            self.cursor_y = y;
                            self.cursor_x = x;
                            found = true;
                            break;
                        }
                    }
                }
                if found { break; }
            }
            
            // Wrap around to beginning
            if !found {
                for y in 0..=self.cursor_y {
                    let line = &self.buffer[y];
                    let end_x = if y == self.cursor_y { self.cursor_x } else { line.len() };
                    
                    for x in 0..end_x {
                        if x + search_chars.len() <= line.len() {
                            let matches = (0..search_chars.len())
                                .all(|i| line[x + i] == search_chars[i]);
                            if matches {
                                self.cursor_y = y;
                                self.cursor_x = x;
                                break;
                            }
                        }
                    }
                }
            }
            
            self.dirty = true;
        }
    }

    fn search_prev(&mut self) {
        if let Some(search) = &self.last_search {
            let search_chars: Vec<char> = search.chars().collect();
            let mut found = false;
            
            // Search backward from current position
            for y in (0..=self.cursor_y).rev() {
                let line = &self.buffer[y];
                let end_x = if y == self.cursor_y {
                    self.cursor_x.saturating_sub(1)
                } else {
                    line.len().saturating_sub(search_chars.len())
                };
                
                for x in (0..=end_x).rev() {
                    if x + search_chars.len() <= line.len() {
                        let matches = (0..search_chars.len())
                            .all(|i| line[x + i] == search_chars[i]);
                        if matches {
                            self.cursor_y = y;
                            self.cursor_x = x;
                            found = true;
                            break;
                        }
                    }
                }
                if found { break; }
            }
            
            self.dirty = true;
        }
    }

    fn page_up(&mut self) {
        let page_size = (self.terminal_height - 2) as usize;
        self.cursor_y = self.cursor_y.saturating_sub(page_size);
        let line_len = self.current_line().len();
        let max_x = if self.mode == Mode::Normal && line_len > 0 && self.config.vim_bindings {
            line_len - 1
        } else {
            line_len
        };
        self.cursor_x = self.cursor_x.min(max_x);
        self.dirty = true;
    }

    fn page_down(&mut self) {
        let page_size = (self.terminal_height - 2) as usize;
        self.cursor_y = (self.cursor_y + page_size).min(self.buffer.len() - 1);
        let line_len = self.current_line().len();
        let max_x = if self.mode == Mode::Normal && line_len > 0 && self.config.vim_bindings {
            line_len - 1
        } else {
            line_len
        };
        self.cursor_x = self.cursor_x.min(max_x);
        self.dirty = true;
    }

    fn insert_char(&mut self, c: char) {
        // Track typing activity
        self.track_typing();
        
        // &mut creates a mutable reference - can modify the line
        let line = &mut self.buffer[self.cursor_y];
        line.insert(self.cursor_x, c);
        self.cursor_x += 1;
        
        // Auto line wrap when reaching terminal width (with some margin)
        let wrap_width = (self.terminal_width - 5) as usize; // Leave some margin
        if self.cursor_x >= wrap_width && c != ' ' {
            // Find last space to break at word boundary
            let mut break_pos = self.cursor_x;
            for i in (0..self.cursor_x).rev() {
                if line[i] == ' ' {
                    break_pos = i + 1;
                    break;
                }
            }
            
            // If no space found or space is too far back, just break at current position
            if break_pos == self.cursor_x || self.cursor_x - break_pos > 20 {
                break_pos = self.cursor_x;
            }
            
            // Move text after break position to new line
            let new_line: Vec<char> = line.drain(break_pos..).collect();
            self.buffer.insert(self.cursor_y + 1, new_line);
            
            // Update cursor position
            self.cursor_y += 1;
            self.cursor_x = self.cursor_x - break_pos;
        }
        
        self.dirty = true;
        self.needs_save = true;
        self.last_save = Instant::now(); // Reset the timer on each change
    }

    fn insert_tab(&mut self) {
        for _ in 0..self.config.tab_size {
            self.insert_char(' ');
        }
    }

    fn insert_newline(&mut self) {
        self.track_typing(); // Track typing activity
        
        let current_line = &mut self.buffer[self.cursor_y];
        let new_line: Vec<char> = current_line.drain(self.cursor_x..).collect();
        self.buffer.insert(self.cursor_y + 1, new_line);
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.dirty = true;
        self.needs_save = true;
        self.last_save = Instant::now();
    }

    fn backspace(&mut self) {
        self.track_typing(); // Track typing activity
        
        if self.cursor_x > 0 {
            self.buffer[self.cursor_y].remove(self.cursor_x - 1);
            self.cursor_x -= 1;
            self.dirty = true;
            self.needs_save = true;
            self.last_save = Instant::now();
        } else if self.cursor_y > 0 {
            let current_line = self.buffer.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.buffer[self.cursor_y].len();
            self.buffer[self.cursor_y].extend(current_line);
            self.dirty = true;
            self.needs_save = true;
            self.last_save = Instant::now();
        }
    }

    fn delete(&mut self) {
        self.track_typing(); // Track typing activity
        
        let line_len = self.current_line().len();
        if self.cursor_x < line_len {
            self.buffer[self.cursor_y].remove(self.cursor_x);
            self.dirty = true;
            self.needs_save = true;
            self.last_save = Instant::now();
        } else if self.cursor_y < self.buffer.len() - 1 {
            let next_line = self.buffer.remove(self.cursor_y + 1);
            self.buffer[self.cursor_y].extend(next_line);
            self.dirty = true;
            self.needs_save = true;
            self.last_save = Instant::now();
        }
    }

    // Returns a reference to the current line
    // &self - immutable borrow (read-only access)
    // &Vec<char> - returns a reference, not ownership
    fn current_line(&self) -> &Vec<char> {
        // & creates a reference to the value
        &self.buffer[self.cursor_y]
    }
    
    fn count_words(&self) -> usize {
        let mut word_count = 0;
        let mut in_word = false;
        
        // & creates iterator over references (doesn't consume self.buffer)
        // Without &, 'for line in self.buffer' would try to move ownership
        for line in &self.buffer {
            for ch in line {
                if ch.is_alphanumeric() {
                    if !in_word {
                        word_count += 1;
                        in_word = true;
                    }
                } else {
                    in_word = false;
                }
            }
            in_word = false; // Reset at end of line
        }
        
        word_count
    }
    
    fn get_stats_file_path(config: &Config) -> PathBuf {
        let today = Local::now();
        let date_str = today.format("%Y-%m-%d").to_string();
        let filename = format!(".stats-{}.toml", date_str);
        Path::new(&config.daily_notes_dir).join(filename)
    }
    
    fn load_typing_time(config: &Config) -> io::Result<Duration> {
        let path = Self::get_stats_file_path(config);
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            if let Ok(stats) = toml::from_str::<DailyStats>(&contents) {
                return Ok(Duration::from_secs(stats.typing_seconds));
            }
        }
        Ok(Duration::from_secs(0))
    }
    
    fn save_typing_time(&self) -> io::Result<()> {
        let path = Self::get_stats_file_path(&self.config);
        let stats = DailyStats {
            typing_seconds: self.get_total_typing_time().as_secs(),
        };
        let toml_str = toml::to_string(&stats).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(&path, toml_str)?;
        Ok(())
    }
    
    fn track_typing(&mut self) {
        let now = Instant::now();
        let typing_timeout = Duration::from_secs(self.config.typing_timeout_seconds);
        
        // If this is the first typing activity or we've been inactive
        if self.typing_session_start.is_none() || now.duration_since(self.last_typing_activity) > typing_timeout {
            self.typing_session_start = Some(now);
        }
        
        self.last_typing_activity = now;
    }
    
    fn get_total_typing_time(&self) -> Duration {
        let mut total = self.accumulated_typing_time;
        
        // Add current session time if actively typing
        if let Some(session_start) = self.typing_session_start {
            let typing_timeout = Duration::from_secs(self.config.typing_timeout_seconds);
            if self.last_typing_activity.elapsed() <= typing_timeout {
                total += self.last_typing_activity.duration_since(session_start);
            }
        }
        
        total
    }

    fn update_offset(&mut self) {
        let visible_height = (self.terminal_height - 2) as usize;
        
        // Vertical scrolling
        if self.cursor_y < self.offset_y {
            self.offset_y = self.cursor_y;
        } else if self.cursor_y >= self.offset_y + visible_height {
            self.offset_y = self.cursor_y - visible_height + 1;
        }
        
        // Horizontal scrolling
        let visible_width = self.terminal_width as usize;
        if self.cursor_x < self.offset_x {
            self.offset_x = self.cursor_x;
        } else if self.cursor_x >= self.offset_x + visible_width {
            self.offset_x = self.cursor_x - visible_width + 1;
        }
    }

    fn render(&mut self) -> io::Result<()> {
        if !self.dirty {
            return Ok(());
        }

        self.update_offset();

        let mut stdout = io::stdout();
        let visible_height = (self.terminal_height - 2) as usize;

        execute!(stdout, Hide)?;

        for y in 0..visible_height {
            execute!(stdout, MoveTo(0, y as u16))?;
            execute!(stdout, Clear(ClearType::CurrentLine))?;

            let file_y = y + self.offset_y;
            if file_y < self.buffer.len() {
                let line = &self.buffer[file_y];
                // Apply horizontal scrolling
                let visible_start = self.offset_x;
                // 'as' performs type casting (u16 to usize)
                // .min() returns the smaller of two values
                let visible_end = (visible_start + self.terminal_width as usize).min(line.len());
                
                if visible_start < line.len() {
                    // Range syntax [start..end] creates a slice
                    // .iter() creates iterator over &char
                    // .collect() builds String from iterator
                    let line_str: String = line[visible_start..visible_end].iter().collect();
                    execute!(stdout, Print(&line_str))?;
                }
            } else {
                execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
                execute!(stdout, Print("~"))?;
                execute!(stdout, ResetColor)?;
            }
        }

        self.render_status_bar()?;

        let screen_y = self.cursor_y - self.offset_y;
        let screen_x = self.cursor_x - self.offset_x;
        execute!(
            stdout,
            MoveTo(screen_x as u16, screen_y as u16),
            Show
        )?;

        stdout.flush()?;
        self.dirty = false;
        Ok(())
    }

    fn render_status_bar(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();
        let y = self.terminal_height - 2;

        // Clear status bar area
        execute!(
            stdout,
            MoveTo(0, y),
            Clear(ClearType::CurrentLine),
            MoveTo(0, y + 1),
            Clear(ClearType::CurrentLine)
        )?;

        // Calculate word count and progress
        let word_count = self.count_words();
        let goal = 500;
        let progress = ((word_count as f32 / goal as f32) * 100.0).min(100.0) as u32;
        
        // Get typing time in minutes
        let typing_time = self.get_total_typing_time();
        let typing_mins = typing_time.as_secs() / 60;
        
        // Create fixed-width formatted strings
        let word_str = format!("{:>4} words", word_count);  // Right-align in 4 chars
        let percent_str = format!("{:>3}%", progress);      // Right-align in 3 chars
        let time_str = format!("{:>3} min", typing_mins);   // Right-align in 3 chars
        
        // Calculate progress bar width - use full terminal width minus the text and spacing
        // Layout: " [progress bar] word_str percent_str · time_str "
        let text_width = 2 + 2 + word_str.len() + 1 + percent_str.len() + 3 + time_str.len() + 1; // brackets, spaces
        let bar_width = (self.terminal_width as usize).saturating_sub(text_width).max(10);
        let filled = (bar_width as f32 * (progress as f32 / 100.0)) as usize;
        let empty = bar_width - filled;
        
        // Create the full-width status line
        // format! macro creates a String using interpolation
        // {} are placeholders filled by subsequent arguments
        let status = format!(" [{}{}] {} {} · {}", 
            "=".repeat(filled),    // String method repeat()
            " ".repeat(empty),
            word_str,
            percent_str,
            time_str
        );
        
        // Set color based on progress
        let color = if word_count >= goal {
            Color::Green
        } else if word_count >= goal * 3 / 4 {
            Color::Yellow
        } else {
            Color::White
        };
        
        execute!(
            stdout,
            MoveTo(0, y),
            SetForegroundColor(color),
            Print(&status),
            ResetColor
        )?;

        // Show command buffer if in command mode
        if self.mode == Mode::Command {
            execute!(
                stdout,
                MoveTo(0, y + 1),
                Print(&self.command_buffer)
            )?;
        }

        Ok(())
    }

    fn save_file(&mut self) -> io::Result<()> {
        if let Some(filename) = &self.filename {
            // Iterator chain pattern - functional programming style
            let content: String = self.buffer
                .iter()                                    // Iterator over &Vec<char>
                .map(|line| line.iter().collect::<String>()) // Transform each line to String
                .collect::<Vec<String>>()                  // Collect into Vec<String>
                .join("\n");                              // Join with newlines
            
            std::fs::write(filename, content)?;
            self.needs_save = false;
            self.last_save = Instant::now();
        }
        Ok(())
    }
    
    fn auto_save(&mut self) -> io::Result<()> {
        self.save_file()
    }

    fn load_file(&mut self, filename: &str) -> io::Result<()> {
        let content = std::fs::read_to_string(filename)?;
        self.buffer = content
            .lines()
            .map(|line| line.chars().collect())
            .collect();
        
        if self.buffer.is_empty() {
            self.buffer.push(Vec::new());
        }
        
        self.filename = Some(filename.to_string());
        
        // Position cursor at end of file
        self.cursor_y = self.buffer.len() - 1;
        self.cursor_x = self.buffer[self.cursor_y].len();
        
        // If the last line has content, add a new line and position cursor there
        if !self.buffer[self.cursor_y].is_empty() {
            self.buffer.push(Vec::new());
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
        
        self.dirty = true;
        Ok(())
    }
}

// Standalone function (not a method) - no self parameter
fn show_stats() -> io::Result<()> {
    let config = Config::load();
    // Path::new creates a Path from a string reference
    let stats_dir = Path::new(&config.daily_notes_dir);
    
    // Collect stats data
    // 'mut' makes variables mutable (variables are immutable by default)
    // _ prefix indicates unused variable (suppresses warning)
    let mut _total_typing_seconds = 0u64; // u64 literal
    let mut total_files = 0;
    // Type annotation with turbofish ::<> syntax
    let mut daily_stats: Vec<(String, u64)> = Vec::new(); // Tuple in Vec
    let mut consecutive_days = 0;
    let today = Local::now();
    
    // Check last 30 days for streak and collect data
    // Range 0..30 creates an iterator from 0 to 29 (exclusive end)
    for days_ago in 0..30 {
        let date = today - chrono::Duration::days(days_ago);
        let date_str = date.format("%Y-%m-%d").to_string();
        let stats_file = stats_dir.join(format!(".stats-{}.toml", date_str));
        let note_file = stats_dir.join(format!("{}.md", date_str));
        
        if stats_file.exists() {
            if let Ok(contents) = fs::read_to_string(&stats_file) {
                // Turbofish syntax ::<Type> specifies generic type parameter
                // Tells from_str what type to deserialize into
                if let Ok(stats) = toml::from_str::<DailyStats>(&contents) {
                    if stats.typing_seconds > 0 {
                        if days_ago as usize == consecutive_days {
                            consecutive_days += 1;
                        }
                        daily_stats.push((date_str.clone(), stats.typing_seconds));
                        _total_typing_seconds += stats.typing_seconds;
                    }
                }
            }
        }
        
        if note_file.exists() {
            total_files += 1;
        }
    }
    
    // Calculate weekly average (last 7 days)
    // Iterator adapter chain - common Rust pattern
    let weekly_typing: u64 = daily_stats.iter()
        .take(7)                    // Take first 7 elements
        .map(|(_, secs)| secs)     // Destructure tuple, ignore first element with _
        .sum();                     // Sum all values (requires type annotation)
    let weekly_avg = weekly_typing / 7;
    
    // Clear screen and display stats
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        Clear(ClearType::All),
        Hide
    )?;
    
    let mut stdout = io::stdout();
    
    // Header
    execute!(
        stdout,
        MoveTo(2, 1),
        SetForegroundColor(Color::Cyan),
        Print("River Writing Statistics"),
        ResetColor
    )?;
    
    // Today's stats
    let today_str = today.format("%Y-%m-%d").to_string();
    let today_typing = daily_stats.iter()
        .find(|(date, _)| date == &today_str)
        .map(|(_, secs)| *secs)
        .unwrap_or(0);
    
    execute!(
        stdout,
        MoveTo(2, 3),
        Print("Today:"),
        MoveTo(20, 3),
        SetForegroundColor(Color::Green),
        Print(format!("{} min", today_typing / 60)),
        ResetColor
    )?;
    
    // Streak
    execute!(
        stdout,
        MoveTo(2, 4),
        Print("Current Streak:"),
        MoveTo(20, 4),
        SetForegroundColor(if consecutive_days > 0 { Color::Yellow } else { Color::DarkGrey }),
        Print(format!("{} days", consecutive_days)),
        ResetColor
    )?;
    
    // Weekly average
    execute!(
        stdout,
        MoveTo(2, 5),
        Print("Weekly Average:"),
        MoveTo(20, 5),
        SetForegroundColor(Color::Blue),
        Print(format!("{} min/day", weekly_avg / 60)),
        ResetColor
    )?;
    
    // Total files
    execute!(
        stdout,
        MoveTo(2, 6),
        Print("Total Notes:"),
        MoveTo(20, 6),
        SetForegroundColor(Color::Magenta),
        Print(format!("{}", total_files)),
        ResetColor
    )?;
    
    // Last 7 days chart
    execute!(
        stdout,
        MoveTo(2, 8),
        SetForegroundColor(Color::Cyan),
        Print("Last 7 Days:"),
        ResetColor
    )?;
    
    let max_mins = daily_stats.iter()
        .take(7)
        .map(|(_, secs)| secs / 60)
        .max()
        .unwrap_or(1)
        .max(1);
    
    // enumerate() adds index to iterator items
    // Pattern (i, (_date, secs)) destructures nested tuples
    for (i, (_date, secs)) in daily_stats.iter().take(7).enumerate() {
        let mins = secs / 60;
        let bar_width = if max_mins > 0 { (mins * 30 / max_mins).min(30) } else { 0 };
        // Method chaining with Option handling
        let day_str = Local::now().checked_sub_signed(chrono::Duration::days(i as i64))
            .map(|d| d.format("%a").to_string())  // Transform Some(date) to Some(string)
            .unwrap_or_default();                  // Use default (empty string) if None
        
        execute!(
            stdout,
            MoveTo(2, 10 + i as u16),
            Print(format!("{:>3}", day_str)),
            MoveTo(6, 10 + i as u16),
            SetForegroundColor(Color::Green),
            Print("█".repeat(bar_width as usize)),
            SetForegroundColor(Color::DarkGrey),
            Print("░".repeat((30 - bar_width) as usize)),
            ResetColor,
            MoveTo(38, 10 + i as u16),
            Print(format!("{:>3} min", mins))
        )?;
    }
    
    // Footer
    execute!(
        stdout,
        MoveTo(2, 20),
        SetForegroundColor(Color::DarkGrey),
        Print("Press any key to exit"),
        ResetColor
    )?;
    
    stdout.flush()?;
    
    // Wait for key press
    event::read()?;
    
    // Clean up
    execute!(
        stdout,
        Show,
        LeaveAlternateScreen
    )?;
    
    Ok(())
}

fn get_daily_note_path(config: &Config) -> io::Result<PathBuf> {
    let today = Local::now();
    let date_str = today.format("%Y-%m-%d").to_string();
    let filename = format!("{}.md", date_str);
    
    let notes_dir = Path::new(&config.daily_notes_dir);
    
    // Create directory if it doesn't exist
    if !notes_dir.exists() {
        fs::create_dir_all(&notes_dir)?;
    }
    
    Ok(notes_dir.join(filename))
}

fn create_daily_note_content() -> String {
    let today = Local::now();
    let date_str = today.format("%A, %B %d, %Y").to_string();
    format!("# {}\n\n", date_str)
}

// Entry point of the program
// main can return Result for error propagation
fn main() -> io::Result<()> {
    // collect() transforms an iterator into a collection
    let args: Vec<String> = std::env::args().collect();
    
    // Check for --stats flag
    // Array indexing with [] - will panic if out of bounds
    if args.len() > 1 && args[1] == "--stats" {
        show_stats()?;
        return Ok(()); // Early return with unit value
    }
    
    let mut editor = Editor::new()?;
    
    if args.len() > 1 {
        // If a file is specified, open it
        editor.load_file(&args[1])?;
    } else {
        // Otherwise, open today's daily note
        let daily_note_path = get_daily_note_path(&editor.config)?;
        
        if !daily_note_path.exists() {
            // Create new daily note with date header
            let content = create_daily_note_content();
            fs::write(&daily_note_path, &content)?;
        }
        
        editor.load_file(&daily_note_path.to_string_lossy())?;
    }
    
    // Last expression without ; is the return value
    editor.run()
}