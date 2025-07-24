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

struct Editor {
    buffer: Vec<Vec<char>>,
    cursor_x: usize,
    cursor_y: usize,
    offset_y: usize,
    terminal_height: u16,
    terminal_width: u16,
    dirty: bool,
    filename: Option<String>,
}

impl Editor {
    fn new() -> io::Result<Self> {
        let (width, height) = terminal::size()?;
        Ok(Editor {
            buffer: vec![Vec::new()],
            cursor_x: 0,
            cursor_y: 0,
            offset_y: 0,
            terminal_height: height,
            terminal_width: width,
            dirty: false,
            filename: None,
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
        match key_event.code {
            KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
            KeyCode::Char('s') if key_event.modifiers.contains(KeyModifiers::CONTROL) => self.save_file()?,
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
                // Handle all character input, including shifted characters
                if !key_event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
                    self.insert_char(c);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.current_line().len();
        }
        self.dirty = true;
    }

    fn move_right(&mut self) {
        let line_len = self.current_line().len();
        if self.cursor_x < line_len {
            self.cursor_x += 1;
        } else if self.cursor_y < self.buffer.len() - 1 {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
        self.dirty = true;
    }

    fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            let line_len = self.current_line().len();
            self.cursor_x = self.cursor_x.min(line_len);
            self.dirty = true;
        }
    }

    fn move_down(&mut self) {
        if self.cursor_y < self.buffer.len() - 1 {
            self.cursor_y += 1;
            let line_len = self.current_line().len();
            self.cursor_x = self.cursor_x.min(line_len);
            self.dirty = true;
        }
    }

    fn move_home(&mut self) {
        self.cursor_x = 0;
        self.dirty = true;
    }

    fn move_end(&mut self) {
        self.cursor_x = self.current_line().len();
        self.dirty = true;
    }

    fn page_up(&mut self) {
        let page_size = (self.terminal_height - 2) as usize;
        self.cursor_y = self.cursor_y.saturating_sub(page_size);
        self.cursor_x = self.cursor_x.min(self.current_line().len());
        self.dirty = true;
    }

    fn page_down(&mut self) {
        let page_size = (self.terminal_height - 2) as usize;
        self.cursor_y = (self.cursor_y + page_size).min(self.buffer.len() - 1);
        self.cursor_x = self.cursor_x.min(self.current_line().len());
        self.dirty = true;
    }

    fn insert_char(&mut self, c: char) {
        let line = &mut self.buffer[self.cursor_y];
        line.insert(self.cursor_x, c);
        self.cursor_x += 1;
        self.dirty = true;
    }

    fn insert_tab(&mut self) {
        for _ in 0..4 {
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
        
        if self.cursor_y < self.offset_y {
            self.offset_y = self.cursor_y;
        } else if self.cursor_y >= self.offset_y + visible_height {
            self.offset_y = self.cursor_y - visible_height + 1;
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
                let line_str: String = line.iter().take(self.terminal_width as usize).collect();
                execute!(stdout, Print(&line_str))?;
            } else {
                execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
                execute!(stdout, Print("~"))?;
                execute!(stdout, ResetColor)?;
            }
        }

        self.render_status_bar()?;

        let screen_y = self.cursor_y - self.offset_y;
        execute!(
            stdout,
            MoveTo(self.cursor_x as u16, screen_y as u16),
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

        let status = format!(
            " {}:{} | {} lines | Ctrl+Q: Quit | Ctrl+S: Save",
            self.cursor_y + 1,
            self.cursor_x + 1,
            self.buffer.len()
        );

        execute!(stdout, Print(&status))?;
        execute!(stdout, ResetColor)?;

        execute!(
            stdout,
            MoveTo(0, y + 1),
            Clear(ClearType::CurrentLine)
        )?;

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
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.dirty = true;
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut editor = Editor::new()?;
    
    if args.len() > 1 {
        editor.load_file(&args[1])?;
    }
    
    editor.run()
}