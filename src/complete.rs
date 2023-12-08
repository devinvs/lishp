use std::fs;
use std::path::Path;

pub fn curr_word(buf: &str, cursor: usize) -> (usize, usize) {
    let chars = buf.chars().collect::<Vec<_>>();
    // move the cursor back until we hit a space or a (
    // Then select the next word as our search prefix
    let mut start = cursor;
    while start != 0 && chars[start] != ' ' && chars[start] != '(' {
        start -= 1;
    }

    start += 1;
    (start, cursor)
}

pub fn complete(buf: &str, cursor: usize) -> Vec<String> {
    let mut options = vec![];
    let (start, cursor) = curr_word(buf, cursor);

    let prefix = buf
        .chars()
        .skip(start)
        .take(cursor - start)
        .collect::<String>();

    options.extend(complete_files(&prefix));

    options
}

fn complete_files(prefix: &str) -> Vec<String> {
    let (base, prefix) = if prefix.starts_with("/") {
        let (base, prefix) = prefix.rsplit_once("/").unwrap();
        let base = if base.is_empty() { "/" } else { base };
        (base, prefix)
    } else {
        if let Some((dir, prefix)) = prefix.rsplit_once("/") {
            (dir, prefix)
        } else {
            ("./", prefix)
        }
    };

    fs::read_dir(base)
        .unwrap()
        .map(|e| e.unwrap())
        .map(|e| {
            if e.file_type().unwrap().is_dir() {
                format!("{}/", e.file_name().to_str().unwrap())
            } else {
                e.file_name().to_str().unwrap().to_string()
            }
        })
        .filter(|s| s.starts_with(prefix))
        .map(|s| Path::new(base).join(s).to_str().unwrap().to_string())
        .map(|s| {
            if s.starts_with("./") {
                s.split_once("./").unwrap().1.to_string()
            } else {
                s
            }
        })
        .map(|s| {
            if s.chars().any(|c| c.is_whitespace()) {
                format!("\"{}\"", s)
            } else {
                s
            }
        })
        .collect()
}
