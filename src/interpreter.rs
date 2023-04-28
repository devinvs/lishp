use std::env::var;
use std::fs::read_dir;
use std::ffi::CString;
use std::collections::HashMap;
use std::collections::LinkedList as List;
use std::path::Path;

use crate::parser::{parse_file, parse_str};

use crate::SExpression;
use crate::builtins::BUILTINS;

use nix::unistd::{
    fork,
    ForkResult,
    execv,
    pipe,
    read,
    close,
    dup2
};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::libc as libc;

pub struct Interpreter {
    // lower level aliases for preprocessing the input text
    pub aliases: HashMap<String, String>,
    // User definitions, literally just use as tree substitutions
    pub defs: HashMap<String, SExpression>,
    // User defined functions, have name, list of args, and then the substituted tree
    pub funcs: HashMap<String, (Vec<String>, SExpression)>,

    // last unique id of a given variable
    pub id: usize,

    // Cached system path
    pub path: Vec<String>,
}

impl Interpreter {
    pub fn load() -> Self {
        let path = var("PATH").expect("Failed to read PATH")
            .split(":")
            .map(|s| s.to_string())
            .collect();

        let mut me = Self {
            id: 0,
            path,
            aliases: HashMap::new(),
            defs: HashMap::new(),
            funcs: HashMap::new(),
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

    pub fn preprocess(&self, s: &str) -> String {
        let mut s = s.to_string();

        for (from, to) in self.aliases.iter() {
            s = s.replace(from, to);
        }

        s
    }

    pub fn eval(&mut self, cmd: &str) -> Result<SExpression, String> {
        // Use aliases as low level text replacements
        let cmd = self.preprocess(cmd);
        // Parse Expression
        let expr = SExpression::parse(&cmd)?;
        // Evaluate Expression
        self.eval_expr(expr, true)
    }

    pub fn eval_expr(&mut self, e: SExpression, root: bool) -> Result<SExpression, String> {
        match e {
            SExpression::Call(mut es) => {
                let func = es.pop_front().ok_or("Empty Call Expression".to_string())?;
                let args = es;

                // If func is a builtin method, run it and print result.
                if let Some(f) = BUILTINS.get(func.ident()) {
                    return f(args, self);
                }

                // If func is in user defined functions then run the subs
                if self.funcs.get(func.ident()).is_some() {
                    let mut fargs = List::new();

                    for arg in args.into_iter() {
                        fargs.push_back(self.eval_expr(arg, false)?);
                    }

                    if let Some((vars, tree)) = self.funcs.get(func.ident()) {
                        let tree = vars.iter()
                            .zip(fargs.iter())
                            .fold(tree.clone(), |t, (var, sub)| {
                                t.replace(var, sub.clone())
                            });
                        return self.eval_expr(tree, false);
                    } else {
                        unreachable!()
                    }
                }

                let (fd_read, fd_write) = pipe().unwrap();

                // Else search path for binary, fork, and exec it with args
                let bin = self.search_path(func.ident());
                if bin.is_none() {
                    return Err(format!("command not found: {}", func))
                }

                let bin = bin.unwrap();

                let mut fargs = vec![];
                fargs.push(CString::new(func.ident()).unwrap());

                for arg in args {
                    fargs.push(CString::new(
                        self.eval_expr(arg, false)?.ident()
                    ).unwrap())
                }
                let args = fargs;

                // Fork, Exec
                match unsafe{fork()} {
                    Ok(ForkResult::Parent { child, .. }) => {
                        let mut out = String::new();
                        let mut buf = [0; 1024];

                        close(fd_write).unwrap();

                        if !root {
                            while let Ok(n) = read(fd_read, &mut buf) {
                                if n == 0 { break; } // EOF
                                out.push_str(&String::from_utf8_lossy(&buf[0..n]));
                            }

                            loop {
                                let status = waitpid(child, Some(WaitPidFlag::WUNTRACED)).unwrap();
                                match status {
                                    WaitStatus::Exited(_, _) | WaitStatus::Signaled(_, _, _) => break,
                                    _ => continue
                                }
                            }

                            let lines = out.lines()
                                .map(|s| SExpression::Atom(s.to_string()))
                                .collect();

                            Ok(SExpression::List(lines))
                        } else {
                            loop {
                                if let Ok(status) = waitpid(child, Some(WaitPidFlag::WUNTRACED)) {
                                    match status {
                                        WaitStatus::Exited(_, _) | WaitStatus::Signaled(_, _, _) => break,
                                        _ => continue
                                    }
                                }
                            }
                            Ok(SExpression::Atom("".into()))
                        }

                    }
                    Ok(ForkResult::Child) => {
                        if !root {
                            dup2(fd_write, 1).unwrap();
                        }
                        close(fd_write).unwrap();
                        close(fd_read).unwrap();

                        execv(&bin, &args).unwrap();
                        unsafe { libc::_exit(0) }
                    }
                    _ => panic!("ah")
                }
            }
            SExpression::Atom(s) => {
                if let Some(expr) = self.defs.get(&s) {
                    Ok(expr.clone())
                } else {
                    Ok(SExpression::Atom(s))
                }
            }
            a => Ok(a),
        }

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
}

