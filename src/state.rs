use std::env::var;
use std::fs::read_dir;
use std::ffi::CString;
use std::collections::HashMap;
use std::path::Path;

use crate::SExpression;

pub struct State {
    // last unique id of a given variable
    pub id: usize,

    // Parsed version of system path
    pub path: Vec<String>,

    // lower level aliases for preprocessing the input text
    pub aliases: HashMap<String, String>,

    // User definitions, literally just use as tree substitutions
    pub defs: HashMap<String, SExpression>,

    // User defined functions, have name, list of args, and then the substituted tree
    pub funcs: HashMap<String, (Vec<String>, SExpression)>
}

impl State {
    pub fn load() -> Self {
        let path = var("PATH").expect("Failed to read PATH")
            .split(":")
            .map(|s| s.to_string())
            .collect();

        Self {
            id: 0,
            path,
            aliases: HashMap::new(),
            defs: HashMap::new(),
            funcs: HashMap::new(),
        }
    }

    pub fn eval(&mut self, cmd: &str) -> Result<SExpression, String> {
        let expr = SExpression::parse(cmd)?;
        expr.eval(self, true)
    }

    pub fn search_path(&self, s: &str) -> Option<CString> {
        // first try to find the file as absolute path
        if let Ok(p) = std::fs::canonicalize(s) {
            let p = Path::new(&p);
            if p.exists() {
                return Some(CString::new(p.to_str().unwrap()).unwrap());
            }
        }

        for dir in self.path.iter() {
            for entry in read_dir(dir).ok()? {
                if let Ok(entry) = entry {
                    if entry.file_name() == s {
                        let p = entry.path();
                        return Some(CString::new(p.to_str().unwrap()).unwrap());
                    }
                }
            }
        }

        None
    }

    pub fn preprocess(&self, s: &str) -> String {
        let mut s = s.to_string();

        for (from, to) in self.aliases.iter() {
            s = s.replace(from, to);
        }

        s
    }
}
