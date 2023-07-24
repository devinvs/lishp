use std::collections::VecDeque;
use std::io::{stdout, Write};

use crossterm::cursor::{position, MoveTo};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{PrintStyledContent, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{execute, QueueableCommand};

use crossterm::style::Print;

pub struct Input {}

impl Input {
    pub fn new() -> Self {
        Input {}
    }

    pub fn readline(&self, prompt: &str) -> Result<String, String> {
        let mut stdout = stdout();

        write!(stdout, "{}", prompt).unwrap();
        stdout.flush().unwrap();

        let (start_col, start_row) = position().unwrap();

        let mut cursor = 1;

        enable_raw_mode().unwrap();
        let mut buf = "()".to_string();

        // In a loop, get a key, process it, and then output the new buffer
        loop {
            stdout
                .queue(MoveTo(start_col, start_row))
                .unwrap()
                .queue(Clear(ClearType::FromCursorDown))
                .unwrap();

            // find the highlighted characters
            let hls = highlight_parens(&buf, cursor as usize);

            for (i, c) in buf.chars().enumerate() {
                stdout
                    .queue(MoveTo(start_col + i as u16, start_row))
                    .unwrap();

                if hls.contains(&i) {
                    stdout
                        .queue(PrintStyledContent(c.to_string().magenta()))
                        .unwrap();
                } else {
                    stdout.queue(Print(c.to_string())).unwrap();
                }
            }

            stdout.flush().unwrap();

            execute!(stdout, MoveTo(start_col + cursor, start_row)).unwrap();

            // Read and process the next key
            match read().unwrap() {
                Event::Key(KeyEvent { code, modifiers }) => match code {
                    // Control characters
                    KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => {
                        disable_raw_mode().unwrap();
                        write!(stdout, "\n").unwrap();
                        return Err("".to_string());
                    }
                    // Navigation
                    KeyCode::Left => cursor = 0.max(cursor - 1),
                    KeyCode::Right => cursor = (buf.len() as u16).min(cursor + 1),

                    // Editing
                    KeyCode::Backspace => {
                        if buf.len() > 0 && cursor > 0 {
                            buf.remove(cursor as usize - 1);
                            cursor -= 1;
                        }
                    }
                    KeyCode::Char('(') => {
                        buf.insert_str(cursor as usize, "()");
                        cursor += 1;
                    }
                    KeyCode::Char(')') => {
                        if buf.chars().nth(cursor as usize).unwrap_or(' ') != ')' {
                            buf.insert(cursor as usize, ')');
                        }

                        cursor += 1;
                    }
                    KeyCode::Char(c) => {
                        buf.insert(cursor as usize, c);
                        cursor += 1;
                    }
                    KeyCode::Enter => {
                        disable_raw_mode().unwrap();
                        write!(stdout, "\n").unwrap();
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(buf)
    }
}

fn highlight_parens(buf: &str, cursor: usize) -> Vec<usize> {
    let mut hls = vec![];
    let mut stack = VecDeque::new();

    for (i, c) in buf.chars().enumerate() {
        if i == cursor {
            break;
        }

        if c == '(' {
            stack.push_front(i);
        }

        if c == ')' {
            stack.pop_front();
        }
    }

    if let Some(i) = stack.pop_front() {
        hls.push(i);
    }

    for (i, c) in buf.chars().enumerate().skip(cursor as usize) {
        if c == ')' {
            hls.push(i);
            break;
        }
    }

    hls
}
