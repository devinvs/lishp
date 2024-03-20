use std::collections::HashMap;
use std::collections::LinkedList as List;
use std::env::var;
use std::ffi::CString;
use std::fs::read_dir;
use std::path::Path;

use crate::parser::{parse_file, parse_str};

use crate::builtins::BUILTINS;
use crate::SExpression;

use nix::libc;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{close, dup2, execv, fork, pipe, read, ForkResult};

pub struct Interpreter {
    // lower level aliases for preprocessing the input text
    pub aliases: HashMap<List<char>, List<List<char>>>,
    // User definitions, literally just use as tree substitutions
    pub defs: HashMap<List<char>, SExpression>,
    // User defined functions, have name, list of args, and then the substituted tree
    pub funcs: HashMap<List<char>, (Vec<List<char>>, SExpression)>,

    // last unique id of a given variable
    pub id: usize,

    // last return code
    pub last_ret_code: i32,
}

impl Interpreter {
    pub fn load() -> Self {
        let mut me = Self {
            id: 0,
            aliases: HashMap::new(),
            defs: HashMap::new(),
            funcs: HashMap::new(),
            last_ret_code: 0,
        };

        // Load prelude
        for expr in parse_str(include_str!("prelude.lisp")) {
            me.eval_expr(expr, false).unwrap();
        }

        // Load .lishprc
        for expr in parse_file("/home/devin/.lishprc") {
            me.eval_expr(expr, false).unwrap();
        }

        me
    }

    pub fn eval(&mut self, cmd: &str) -> Result<SExpression, String> {
        // Parse Expression
        let expr = SExpression::parse(&cmd, &self.aliases)?;
        // Evaluate Expression
        self.eval_expr(expr, true)
    }

    pub fn eval_expr(&mut self, e: SExpression, root: bool) -> Result<SExpression, String> {
        match e {
            SExpression::Call(mut es) => {
                let func = es
                    .pop_front()
                    .ok_or("Empty Call Expression".to_string())?
                    .ident();
                let func_name: String = func.iter().collect();
                let args = es;

                // If func is a builtin method, run it and print result.
                if let Some(f) = BUILTINS.get(func_name.as_str()) {
                    return f(args, self);
                }

                // If func is in user defined functions then run the subs
                if self.funcs.get(&func).is_some() {
                    let mut fargs = List::new();

                    for arg in args.into_iter() {
                        fargs.push_back(self.eval_expr(arg, false)?);
                    }

                    if let Some((vars, tree)) = self.funcs.get(&func) {
                        let tree = vars
                            .iter()
                            .zip(fargs.iter())
                            .fold(tree.clone(), |t, (var, sub)| t.replace(var, sub.clone()));
                        return self.eval_expr(tree, false);
                    } else {
                        unreachable!()
                    }
                }

                let (fd_read, fd_write) = pipe().unwrap();

                // Else search path for binary, fork, and exec it with args
                let bin = self.search_path(&func_name);
                if bin.is_none() {
                    return Err(format!("command not found: {}", func_name));
                }

                let bin = bin.unwrap();

                let mut fargs = vec![];
                fargs.push(CString::new(func_name).unwrap());

                for arg in args {
                    fargs.push(
                        CString::new(
                            self.eval_expr(arg, false)?
                                .ident()
                                .iter()
                                .collect::<String>(),
                        )
                        .unwrap(),
                    )
                }
                let args = fargs;

                // Fork, Exec
                match unsafe { fork() } {
                    Ok(ForkResult::Parent { child, .. }) => {
                        let mut out = String::new();
                        let mut buf = [0; 1024];

                        close(fd_write).unwrap();

                        if !root {
                            while let Ok(n) = read(fd_read, &mut buf) {
                                if n == 0 {
                                    break;
                                } // EOF
                                out.push_str(&String::from_utf8_lossy(&buf[0..n]));
                            }

                            loop {
                                let status = waitpid(child, Some(WaitPidFlag::WUNTRACED)).unwrap();
                                match status {
                                    WaitStatus::Exited(_, _) => {
                                        break;
                                    }
                                    WaitStatus::Signaled(_, _, _) => break,
                                    _ => continue,
                                }
                            }

                            let lines = out
                                .lines()
                                .map(|s| SExpression::Atom(s.chars().collect()))
                                .collect();

                            Ok(SExpression::List(lines))
                        } else {
                            loop {
                                if let Ok(status) = waitpid(child, Some(WaitPidFlag::WUNTRACED)) {
                                    match status {
                                        WaitStatus::Exited(_, exit) => {
                                            self.last_ret_code = exit;
                                            break;
                                        }
                                        WaitStatus::Signaled(_, _, _) => break,
                                        _ => continue,
                                    }
                                }
                            }
                            Ok(SExpression::Atom(List::new()))
                        }
                    }
                    Ok(ForkResult::Child) => {
                        if !root {
                            dup2(fd_write, 1).unwrap();
                        }
                        close(fd_write).unwrap();
                        close(fd_read).unwrap();

                        if let Err(e) = execv(&bin, &args) {
                            eprintln!("error: {e}");
                        }
                        unsafe { libc::_exit(0) }
                    }
                    _ => panic!("ah"),
                }
            }
            SExpression::Atom(s) => {
                if s == "$?".chars().collect() {
                    Ok(SExpression::Atom(
                        self.last_ret_code.to_string().chars().collect(),
                    ))
                } else if let Some(expr) = self.defs.get(&s) {
                    Ok(expr.clone())
                } else {
                    Ok(SExpression::Atom(s))
                }
            }
            a => Ok(a),
        }
    }

    pub fn search_path(&self, s: &str) -> Option<CString> {
        let path: Vec<_> = var("PATH")
            .expect("Failed to read PATH")
            .split(":")
            .map(|s| s.to_string())
            .collect();

        // first try to find the file as absolute path
        if let Ok(p) = std::fs::canonicalize(s) {
            let p = Path::new(&p);
            if p.exists() {
                return Some(CString::new(p.to_str().unwrap()).unwrap());
            }
        }

        for dir in path.iter() {
            if let Ok(entries) = read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.file_name() == s {
                            let p = entry.path();
                            return Some(CString::new(p.to_str().unwrap()).unwrap());
                        }
                    }
                }
            }
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
