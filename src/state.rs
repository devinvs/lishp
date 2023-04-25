use std::env::var;
use std::fs::read_dir;
use std::ffi::CString;
use std::collections::HashMap;

use crate::parser::SExpression;

pub struct State {
    // last unique id of a given variable
    pub id: usize,

    // Parsed version of system path
    pub path: Vec<String>,

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
            defs: HashMap::new(),
            funcs: HashMap::new(),
        }
    }

    pub fn search_path(&self, s: &str) -> Option<CString> {
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
}
