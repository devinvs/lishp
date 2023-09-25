use std::collections::VecDeque;
use std::io::{stdout, Write};

use crossterm::cursor::{position, MoveTo};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{PrintStyledContent, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{execute, QueueableCommand};

use crossterm::style::Print;

use crate::complete::{complete, curr_word};

#[derive(Debug, Clone)]
pub enum History {
    Nil,
    Cons(String, Box<History>),
}

pub struct Input {}

impl Input {
    pub fn new() -> Self {
        Input {}
    }

    pub fn readline(&self, prompt: &str, history: History) -> Result<String, String> {
        self.readline_buf(prompt, history, History::Nil, "()".to_string(), 1)
    }

    pub fn readline_buf(
        &self,
        prompt: &str,
        mut history: History,
        mut history_prev: History,
        mut buf: String,
        mut cursor: u16,
    ) -> Result<String, String> {
        let mut stdout = stdout();

        write!(stdout, "{}", prompt).unwrap();
        stdout.flush().unwrap();

        let (start_col, start_row) = position().unwrap();

        enable_raw_mode().unwrap();
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
                Event::Key(KeyEvent { code, modifiers }) => match (code, modifiers) {
                    // autocomplete
                    (KeyCode::Tab, _) => {
                        let cs = complete(&buf, cursor as usize);

                        if cs.len() == 0 {
                            continue;
                        }

                        let prefix = cs[0].clone();
                        let mut plen = prefix.len();

                        for s in cs.iter() {
                            plen = prefix
                                .chars()
                                .zip(s.chars())
                                .take_while(|(a, b)| a == b)
                                .count()
                                .min(plen)
                        }

                        if plen > 0 {
                            let c: String = prefix.chars().take(plen).collect();
                            let (start, end) = curr_word(&buf, cursor as usize);

                            buf = format!(
                                "{}{}{}",
                                buf.chars().take(start).collect::<String>(),
                                c,
                                buf.chars().skip(end).collect::<String>()
                            );
                            cursor = (start + c.len()) as u16;
                        }

                        if cs.len() > 1 && cs.len() < 25 {
                            disable_raw_mode().unwrap();
                            write!(stdout, "\n").unwrap();
                            println!(
                                "{}",
                                cs.into_iter()
                                    .map(|s| {
                                        if s.ends_with("/") {
                                            let (a, _) = s.rsplit_once("/").unwrap();
                                            if let Some((_, a)) = a.rsplit_once("/") {
                                                format!("{a}/")
                                            } else {
                                                format!("{a}/")
                                            }
                                        } else if let Some((_, a)) = s.rsplit_once("/") {
                                            a.to_string()
                                        } else {
                                            s
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            );
                            return self.readline_buf(prompt, history, history_prev, buf, cursor);
                        }
                    }
                    // Control characters
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        disable_raw_mode().unwrap();
                        write!(stdout, "\n").unwrap();
                        return Err("".to_string());
                    }
                    // Navigation
                    (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                        cursor = 0.max(cursor - 1)
                    }
                    (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                        cursor = (buf.len() as u16).min(cursor + 1)
                    }
                    (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                        cursor = 0;
                    }
                    (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                        cursor = buf.len() as u16;
                    }
                    // history
                    (KeyCode::Up, _) => match history.clone() {
                        History::Nil => {}
                        History::Cons(val, rest) => {
                            buf = val.clone();
                            history = *rest;
                            history_prev = History::Cons(val, Box::new(history_prev));
                        }
                    },
                    // Editing
                    (KeyCode::Backspace, _) => {
                        if buf.len() > 0 && cursor > 0 {
                            buf.remove(cursor as usize - 1);
                            cursor -= 1;
                        }
                    }
                    (KeyCode::Char('('), _) => {
                        // Count number of ( and )
                        // if ( < ) insert only (
                        // else insert ()
                        let l_count = buf.chars().filter(|c| *c == '(').count();
                        let r_count = buf.chars().filter(|c| *c == ')').count();

                        if l_count < r_count {
                            buf.insert(cursor as usize, '(');
                        } else {
                            buf.insert_str(cursor as usize, "()");
                        }

                        cursor += 1;
                    }
                    (KeyCode::Char(')'), _) => {
                        if buf.chars().nth(cursor as usize).unwrap_or(' ') != ')' {
                            buf.insert(cursor as usize, ')');
                        }

                        cursor += 1;
                    }
                    (KeyCode::Char(c), _) => {
                        buf.insert(cursor as usize, c);
                        cursor += 1;
                    }
                    (KeyCode::Enter, _) => {
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

        for (i, c) in buf.chars().enumerate().skip(cursor as usize) {
            if c == ')' {
                hls.push(i);
                break;
            }
        }
    }

    hls
}
