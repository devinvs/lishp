use rustyline::completion::{Pair, Candidate, Completer};
use rustyline::{Result, Context};

use std::borrow::Cow::{self, Borrowed, Owned};
use std::fs;
use std::path::{self, Path};
use crate::SExpression;

#[derive(Default)]
struct InputValidator;

use rustyline::validate::{ValidationContext, ValidationResult};
use rustyline::{Completer, Helper, Highlighter, Hinter, Validator};

impl rustyline::validate::Validator for InputValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        match SExpression::parse(ctx.input()) {
            Ok(_) => Ok(ValidationResult::Valid(None)),
            Err(_) => Ok(ValidationResult::Incomplete)
        }
    }
}

#[derive(Completer, Helper, Highlighter, Hinter, Validator, Default)]
pub struct InputHelper {
    #[rustyline(Validator)]
    validator: InputValidator,
    #[rustyline(Completer)]
    completer: FilenameCompleter
}

pub struct FilenameCompleter {
    break_chars: fn(char) -> bool,
    double_quotes_special_chars: fn(char) -> bool,
}

const DOUBLE_QUOTES_ESCAPE_CHAR: Option<char> = Some('\\');

const fn default_break_chars(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '"' | '\\' | '(' | ')')
}
const ESCAPE_CHAR: Option<char> = Some('\\');
// In double quotes, not all break_chars need to be escaped
// https://www.gnu.org/software/bash/manual/html_node/Double-Quotes.html
const fn double_quotes_special_chars(c: char) -> bool { matches!(c, '"' | '$' | '\\' | '`') }


/// Kind of quote.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Quote {
    /// Double quote: `"`
    Double,
    /// Single quote: `'`
    Single,
    /// No quote
    None,
}

impl FilenameCompleter {
    /// Constructor
    #[must_use]
    pub fn new() -> Self {
        Self {
            break_chars: default_break_chars,
            double_quotes_special_chars,
        }
    }

    /// Takes the currently edited `line` with the cursor `pos`ition and
    /// returns the start position and the completion candidates for the
    /// partial path to be completed.
    pub fn complete_path(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>)> {
        let (start, mut matches) = self.complete_path_unsorted(line, pos)?;
        #[allow(clippy::unnecessary_sort_by)]
        matches.sort_by(|a, b| a.display().cmp(b.display()));
        Ok((start, matches))
    }

    /// Similar to [`Self::complete_path`], but the returned paths are unsorted.
    pub fn complete_path_unsorted(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>)> {
        let (start, path, esc_char, break_chars, quote) =
            if let Some((idx, quote)) = find_unclosed_quote(&line[..pos]) {
                let start = idx + 1;
                if quote == Quote::Double {
                    (
                        start,
                        unescape(&line[start..pos], DOUBLE_QUOTES_ESCAPE_CHAR),
                        DOUBLE_QUOTES_ESCAPE_CHAR,
                        self.double_quotes_special_chars,
                        quote,
                    )
                } else {
                    (
                        start,
                        Borrowed(&line[start..pos]),
                        None,
                        self.break_chars,
                        quote,
                    )
                }
            } else {
                let (start, path) = extract_word(line, pos, ESCAPE_CHAR, self.break_chars);
                let path = unescape(path, ESCAPE_CHAR);
                (start, path, ESCAPE_CHAR, self.break_chars, Quote::None)
            };
        let matches = filename_complete(&path, esc_char, break_chars, quote);
        Ok((start, matches))
    }
}

impl Default for FilenameCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for FilenameCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        self.complete_path(line, pos)
    }
}

/// Remove escape char
#[must_use]
pub fn unescape(input: &str, esc_char: Option<char>) -> Cow<'_, str> {
    let esc_char = if let Some(c) = esc_char {
        c
    } else {
        return Borrowed(input);
    };
    if !input.chars().any(|c| c == esc_char) {
        return Borrowed(input);
    }
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == esc_char {
            if let Some(ch) = chars.next() {
                if cfg!(windows) && ch != '"' {
                    // TODO Validate: only '"' ?
                    result.push(esc_char);
                }
                result.push(ch);
            } else if cfg!(windows) {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }
    Owned(result)
}

/// Escape any `break_chars` in `input` string with `esc_char`.
/// For example, '/User Information' becomes '/User\ Information'
/// when space is a breaking char and '\\' the escape char.
#[must_use]
pub fn escape(
    mut input: String,
    esc_char: Option<char>,
    is_break_char: fn(char) -> bool,
    quote: Quote,
) -> String {
    if quote == Quote::Single {
        return input; // no escape in single quotes
    }
    let n = input.chars().filter(|c| is_break_char(*c)).count();
    if n == 0 {
        return input; // no need to escape
    }
    let esc_char = if let Some(c) = esc_char {
        c
    } else {
        if cfg!(windows) && quote == Quote::None {
            input.insert(0, '"'); // force double quote
            return input;
        }
        return input;
    };
    let mut result = String::with_capacity(input.len() + n);

    for c in input.chars() {
        if is_break_char(c) {
            result.push(esc_char);
        }
        result.push(c);
    }
    result
}

fn filename_complete(
    path: &str,
    esc_char: Option<char>,
    is_break_char: fn(char) -> bool,
    quote: Quote,
) -> Vec<Pair> {
    #[cfg(feature = "with-dirs")]
    use home::home_dir;
    use std::env::current_dir;

    let sep = path::MAIN_SEPARATOR;
    let (dir_name, file_name) = match path.rfind(sep) {
        Some(idx) => path.split_at(idx + sep.len_utf8()),
        None => ("", path),
    };

    let dir_path = Path::new(dir_name);
    let dir = if dir_path.starts_with("~") {
        // ~[/...]
        #[cfg(feature = "with-dirs")]
        {
            if let Some(home) = home_dir() {
                match dir_path.strip_prefix("~") {
                    Ok(rel_path) => home.join(rel_path),
                    _ => home,
                }
            } else {
                dir_path.to_path_buf()
            }
        }
        #[cfg(not(feature = "with-dirs"))]
        {
            dir_path.to_path_buf()
        }
    } else if dir_path.is_relative() {
        // TODO ~user[/...] (https://crates.io/crates/users)
        if let Ok(cwd) = current_dir() {
            cwd.join(dir_path)
        } else {
            dir_path.to_path_buf()
        }
    } else {
        dir_path.to_path_buf()
    };

    let mut entries: Vec<Pair> = Vec::new();

    // if dir doesn't exist, then don't offer any completions
    if !dir.exists() {
        return entries;
    }

    // if any of the below IO operations have errors, just ignore them
    if let Ok(read_dir) = dir.read_dir() {
        let file_name = normalize(file_name);
        for entry in read_dir.flatten() {
            if let Some(s) = entry.file_name().to_str() {
                let ns = normalize(s);
                if ns.starts_with(file_name.as_ref()) {
                    if let Ok(metadata) = fs::metadata(entry.path()) {
                        let mut path = String::from(dir_name) + s;
                        if metadata.is_dir() {
                            path.push(sep);
                        }
                        entries.push(Pair {
                            display: String::from(s),
                            replacement: escape(path, esc_char, is_break_char, quote),
                        });
                    } // else ignore PermissionDenied
                }
            }
        }
    }
    entries
}

#[cfg(not(any(windows, target_os = "macos")))]
fn normalize(s: &str) -> Cow<str> {
    Cow::Borrowed(s)
}

/// Given a `line` and a cursor `pos`ition,
/// try to find backward the start of a word.
/// Return (0, `line[..pos]`) if no break char has been found.
/// Return the word and its start position (idx, `line[idx..pos]`) otherwise.
#[must_use]
pub fn extract_word(
    line: &str,
    pos: usize,
    esc_char: Option<char>,
    is_break_char: fn(char) -> bool,
) -> (usize, &str) {
    let line = &line[..pos];
    if line.is_empty() {
        return (0, line);
    }
    let mut start = None;
    for (i, c) in line.char_indices().rev() {
        if let (Some(esc_char), true) = (esc_char, start.is_some()) {
            if esc_char == c {
                // escaped break char
                start = None;
                continue;
            }
            break;
        }
        if is_break_char(c) {
            start = Some(i + c.len_utf8());
            if esc_char.is_none() {
                break;
            } // else maybe escaped...
        }
    }

    match start {
        Some(start) => (start, &line[start..]),
        None => (0, line),
    }
}

/// Returns the longest common prefix among all `Candidate::replacement()`s.
pub fn longest_common_prefix<C: Candidate>(candidates: &[C]) -> Option<&str> {
    if candidates.is_empty() {
        return None;
    } else if candidates.len() == 1 {
        return Some(candidates[0].replacement());
    }
    let mut longest_common_prefix = 0;
    'o: loop {
        for (i, c1) in candidates.iter().enumerate().take(candidates.len() - 1) {
            let b1 = c1.replacement().as_bytes();
            let b2 = candidates[i + 1].replacement().as_bytes();
            if b1.len() <= longest_common_prefix
                || b2.len() <= longest_common_prefix
                || b1[longest_common_prefix] != b2[longest_common_prefix]
            {
                break 'o;
            }
        }
        longest_common_prefix += 1;
    }
    let candidate = candidates[0].replacement();
    while !candidate.is_char_boundary(longest_common_prefix) {
        longest_common_prefix -= 1;
    }
    if longest_common_prefix == 0 {
        return None;
    }
    Some(&candidate[0..longest_common_prefix])
}

#[derive(Eq, PartialEq)]
enum ScanMode {
    DoubleQuote,
    Escape,
    EscapeInDoubleQuote,
    Normal,
    SingleQuote,
}

/// try to find an unclosed single/double quote in `s`.
/// Return `None` if no unclosed quote is found.
/// Return the unclosed quote position and if it is a double quote.
fn find_unclosed_quote(s: &str) -> Option<(usize, Quote)> {
    let char_indices = s.char_indices();
    let mut mode = ScanMode::Normal;
    let mut quote_index = 0;
    for (index, char) in char_indices {
        match mode {
            ScanMode::DoubleQuote => {
                if char == '"' {
                    mode = ScanMode::Normal;
                } else if char == '\\' {
                    // both windows and unix support escape in double quote
                    mode = ScanMode::EscapeInDoubleQuote;
                }
            }
            ScanMode::Escape => {
                mode = ScanMode::Normal;
            }
            ScanMode::EscapeInDoubleQuote => {
                mode = ScanMode::DoubleQuote;
            }
            ScanMode::Normal => {
                if char == '"' {
                    mode = ScanMode::DoubleQuote;
                    quote_index = index;
                } else if char == '\\' && cfg!(not(windows)) {
                    mode = ScanMode::Escape;
                } else if char == '\'' && cfg!(not(windows)) {
                    mode = ScanMode::SingleQuote;
                    quote_index = index;
                }
            }
            ScanMode::SingleQuote => {
                if char == '\'' {
                    mode = ScanMode::Normal;
                } // no escape in single quotes
            }
        };
    }
    if ScanMode::DoubleQuote == mode || ScanMode::EscapeInDoubleQuote == mode {
        return Some((quote_index, Quote::Double));
    } else if ScanMode::SingleQuote == mode {
        return Some((quote_index, Quote::Single));
    }
    None
}
