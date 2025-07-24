use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::io::{self, Write};
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::fs;
use chrono::Local;

mod config;
use config::Config;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

struct Editor {
    buffer: Vec<Vec<char>>,
    cursor_x: usize,
    cursor_y: usize,
    offset_y: usize,
    offset_x: usize,
    terminal_height: u16,
    terminal_width: u16,
    dirty: bool,
    filename: Option<String>,
    mode: Mode,
    command_buffer: String,
    clipboard: Vec<Vec<char>>,
    last_search: Option<String>,
    config: Config,
}

impl Editor {
    fn new() -> io::Result<Self> {
        let (width, height) = terminal::size()?;
        let config = Config::load();
        let mode = if config.vim_bindings {
            Mode::Normal
        } else {
            Mode::Insert
        };
        
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
        })
    }

    fn run(&mut self) -> io::Result<()> {
        self.enter_raw_mode()?;
        
        loop {
            self.render()?;
            
            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key_event) = event::read()? {
                    if self.handle_key_event(key_event)? {
                        break;
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

    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<bool> {
        if self.config.vim_bindings {
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
        match key_event.code {
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
            KeyCode::Char(c) => {
                if !key_event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
                    self.insert_char(c);
                }
            }
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
                "w" => self.save_file()?,
                "wq" => {
                    self.save_file()?;
                    return Ok(true);
                }
                _ => {}
            }
        }
        Ok(false)
    }

    fn last_key_was(&self, _c: char) -> bool {
        // Simplified for now - in a real implementation, we'd track the last key
        true
    }

    fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 && (self.mode == Mode::Insert || !self.config.vim_bindings) {
            self.cursor_y -= 1;
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
        if self.cursor_x < self.current_line().len() {
            self.buffer[self.cursor_y].remove(self.cursor_x);
            if self.cursor_x > 0 && self.cursor_x == self.current_line().len() && self.config.vim_bindings {
                self.cursor_x -= 1;
            }
            self.dirty = true;
        }
    }

    fn delete_line(&mut self) {
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
    }

    fn yank_line(&mut self) {
        self.clipboard = vec![self.buffer[self.cursor_y].clone()];
    }

    fn paste_after(&mut self) {
        if !self.clipboard.is_empty() {
            for (i, line) in self.clipboard.iter().enumerate() {
                self.buffer.insert(self.cursor_y + 1 + i, line.clone());
            }
            self.cursor_y += 1;
            self.cursor_x = 0;
            self.dirty = true;
        }
    }

    fn paste_before(&mut self) {
        if !self.clipboard.is_empty() {
            for (i, line) in self.clipboard.iter().enumerate() {
                self.buffer.insert(self.cursor_y + i, line.clone());
            }
            self.cursor_x = 0;
            self.dirty = true;
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
    }

    fn insert_tab(&mut self) {
        for _ in 0..self.config.tab_size {
            self.insert_char(' ');
        }
    }

    fn insert_newline(&mut self) {
        let current_line = &mut self.buffer[self.cursor_y];
        let new_line: Vec<char> = current_line.drain(self.cursor_x..).collect();
        self.buffer.insert(self.cursor_y + 1, new_line);
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.dirty = true;
    }

    fn backspace(&mut self) {
        if self.cursor_x > 0 {
            self.buffer[self.cursor_y].remove(self.cursor_x - 1);
            self.cursor_x -= 1;
            self.dirty = true;
        } else if self.cursor_y > 0 {
            let current_line = self.buffer.remove(self.cursor_y);
            self.cursor_y -= 1;
            self.cursor_x = self.buffer[self.cursor_y].len();
            self.buffer[self.cursor_y].extend(current_line);
            self.dirty = true;
        }
    }

    fn delete(&mut self) {
        let line_len = self.current_line().len();
        if self.cursor_x < line_len {
            self.buffer[self.cursor_y].remove(self.cursor_x);
            self.dirty = true;
        } else if self.cursor_y < self.buffer.len() - 1 {
            let next_line = self.buffer.remove(self.cursor_y + 1);
            self.buffer[self.cursor_y].extend(next_line);
            self.dirty = true;
        }
    }

    fn current_line(&self) -> &Vec<char> {
        &self.buffer[self.cursor_y]
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
                let visible_end = (visible_start + self.terminal_width as usize).min(line.len());
                
                if visible_start < line.len() {
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

        execute!(
            stdout,
            MoveTo(0, y),
            SetBackgroundColor(Color::DarkGrey),
            SetForegroundColor(Color::White),
            Clear(ClearType::CurrentLine)
        )?;

        let filename_str = self.filename.as_ref()
            .map(|f| f.as_str())
            .unwrap_or("[No Name]");

        let status = if self.config.vim_bindings {
            let mode_str = match self.mode {
                Mode::Normal => "NORMAL",
                Mode::Insert => "INSERT",
                Mode::Command => "COMMAND",
            };
            format!(
                " {} | {} | {}:{} | {} lines",
                mode_str,
                filename_str,
                self.cursor_y + 1,
                self.cursor_x + 1,
                self.buffer.len()
            )
        } else {
            format!(
                " {} | {}:{} | {} lines | Ctrl+Q: Quit",
                filename_str,
                self.cursor_y + 1,
                self.cursor_x + 1,
                self.buffer.len()
            )
        };

        execute!(stdout, Print(&status))?;
        execute!(stdout, ResetColor)?;

        execute!(
            stdout,
            MoveTo(0, y + 1),
            Clear(ClearType::CurrentLine)
        )?;

        if self.mode == Mode::Command {
            execute!(stdout, Print(&self.command_buffer))?;
        }

        Ok(())
    }

    fn save_file(&mut self) -> io::Result<()> {
        if let Some(filename) = &self.filename {
            let content: String = self.buffer
                .iter()
                .map(|line| line.iter().collect::<String>())
                .collect::<Vec<String>>()
                .join("\n");
            
            std::fs::write(filename, content)?;
        }
        Ok(())
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

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
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
    
    editor.run()
}