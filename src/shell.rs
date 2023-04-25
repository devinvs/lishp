use std::collections::HashMap;
use std::ffi::CString;

use crate::parser::SExpression;
use crate::state::State;


use nix::unistd::{
    fork,
    ForkResult,
    execv,
    pipe,
    read,
    close,
    dup2
};
use nix::sys::wait::waitpid;
use nix::libc as libc;

use lazy_static::lazy_static;

type Func = fn(Box<dyn Iterator<Item = SExpression>>, &mut State) -> Result<SExpression, String>;

mod builtin {
    use std::cmp::Ordering;
    use std::env::set_current_dir;
    use crate::parser::SExpression;
    use crate::state::State;

    type BinNum = fn(f64, f64) -> f64;
    type BinCmp = fn(f64, f64) -> bool;

    fn to_f64(e: &SExpression) -> Result<f64, String> {
        e.ident().parse::<f64>()
            .map_err(|_| format!("{e} is not a number"))
    }

    pub fn fold_nums(args: Box<dyn Iterator<Item = SExpression>>, init: f64, f: BinNum, s: &mut State) -> Result<SExpression, String> {
        let mut accum = init;

        for arg in args {
            let x = to_f64(&arg.eval(s, false)?)?;
            accum = f(accum, x);
        }

        Ok(SExpression::Atom(accum.to_string()))
    }

    fn bin_num(x: SExpression, y: SExpression, f: BinNum, s: &mut State) -> Result<SExpression, String> {
        Ok(SExpression::Atom(f(
            to_f64(&x.eval(s, false)?)?,
            to_f64(&y.eval(s, false)?)?).to_string()))
    }

    pub fn add(args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        fold_nums(args, 0.0, |a, b| a+b, s)
    }

    pub fn sub(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            bin_num(x, y, |a, b| a - b, s)
        } else {
            Err("sub requires two arguments".to_string())
        }
    }

    pub fn mul(args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        fold_nums(args, 1.0, |a, b| a*b, s)
    }

    pub fn div(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            bin_num(x, y, |a, b| a / b, s)
        } else {
            Err("div requires two arguments".to_string())
        }
    }

    pub fn modulus(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            bin_num(x, y, |a, b| a % b, s)
        } else {
            Err("mod requires two arguments".to_string())
        }
    }

    pub fn pow(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            bin_num(x, y, |a, b| a.powf(b), s)
        } else {
            Err("pow requires two arguments".to_string())
        }
    }

    fn bin_cmp(x: SExpression, y: SExpression, f: BinCmp, s: &mut State) -> Result<SExpression, String> {
        Ok(SExpression::Atom(f(
                to_f64(&x.eval(s, false)?)?,
                to_f64(&y.eval(s, false)?)?).to_string()))
    }

    pub fn not(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let Some(e) = args.next() {
            match e.eval(s, false)? {
                SExpression::Atom(s) => {
                    match s.as_str() {
                        "true" => Ok(SExpression::Atom("false".to_string())),
                        "false" => Ok(SExpression::Atom("true".to_string())),
                        _ => Err("not expects a boolean argument".to_string())
                    }
                }
                _ => Err("not expects a boolean argument".to_string())
            }
        } else {
            Err("not requires one argument".to_string())
        }
    }

    pub fn or(args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        let mut accum = false;

        for arg in args {
            let x = arg.eval(s, false)?;
            accum = accum || x.ident() == "true";
        }

        Ok(SExpression::Atom(accum.to_string()))
    }

    pub fn and(args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        let mut accum = true;

        for arg in args {
            let x = arg.eval(s, false)?;
            accum = accum && x.ident() == "true";
        }

        Ok(SExpression::Atom(accum.to_string()))
    }

    pub fn lt(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            bin_cmp(x, y, |a, b| a.partial_cmp(&b).unwrap() == Ordering::Less, s)
        } else {
            Err("lt requires two arguments".to_string())
        }
    }

    pub fn gt(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            bin_cmp(x, y, |a, b| a.partial_cmp(&b).unwrap() == Ordering::Greater, s)
        } else {
            Err("gt requires two arguments".to_string())
        }
    }

    pub fn leq(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            bin_cmp(x, y, |a, b| a==b || a.partial_cmp(&b).unwrap() == Ordering::Less, s)
        } else {
            Err("leq requires two arguments".to_string())
        }
    }

    pub fn geq(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            bin_cmp(x, y, |a, b| a==b || a.partial_cmp(&b).unwrap() == Ordering::Greater, s)
        } else {
            Err("geq requires two arguments".to_string())
        }
    }

    pub fn eq(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(y)) = (args.next(), args.next()) {
            let x = x.eval(s, false);
            let y = y.eval(s, false);

            Ok(SExpression::Atom((x==y).to_string()))
        } else {
            Err("eq requires two arguments".to_string())
        }
    }

    pub fn ifs(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(cond), Some(t), Some(f)) = (args.next(), args.next(), args.next()) {
            if cond.eval(s, false)?.ident() == "true" {
                t.eval(s, false)
            } else {
                f.eval(s, false)
            }
        } else {
            Err("if requires three arguments".to_string())
        }
    }

    pub fn first(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let Some(e) = args.next() {
            match e.eval(s, false)? {
                SExpression::List(es) => {
                    if let Some(e) = es.into_iter().next() {
                        Ok(e)
                    } else {
                        Err("tried to call first on empty list".to_string())
                    }
                }
                SExpression::Atom(s) => {
                    if let Some(c) = s.chars().next() {
                        Ok(SExpression::Atom(c.to_string()))
                    } else {
                        Err("tried to call first on empty string".to_string())
                    }
                }
                _ => unreachable!()
            }
        } else {
            Err("first requires one argument".to_string())
        }
    }

    pub fn rest(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let Some(e) = args.next() {
            match e.eval(s, false)? {
                SExpression::List(es) => {
                    let mut i = es.into_iter();
                    i.next();

                    Ok(SExpression::List(i.collect()))
                }
                SExpression::Atom(s) => {
                    let mut i = s.chars();
                    i.next();
                    Ok(SExpression::Atom(i.collect()))
                }
                _ => unreachable!()
            }
        } else {
            Err("rest requires one argument".to_string())
        }
    }

    pub fn list(args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        let mut l = Vec::new();

        for arg in args {
            l.push(arg.eval(s, false)?);
        }

        Ok(SExpression::List(l))
    }

    pub fn def(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(SExpression::Atom(name)), Some(val)) = (args.next(), args.next()) {
            s.defs.insert(name.clone(), val);
            Ok(SExpression::Atom(format!("defined {name}")))
        } else {
            Err("def requires two arguments".to_string())
        }
    }

    pub fn defun(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(SExpression::Atom(name)), Some(vars), Some(mut tree)) = (args.next(), args.next(), args.next()) {
            let mut vs = Vec::new();

            // rename variables to be unique...
            for var in vars.list() {
                let new = format!("#{}#{}#", var, s.id);
                vs.push(new.clone());
                tree = tree.replace(&var, SExpression::Atom(new));
                s.id += 1;
            }

            s.funcs.insert(name.clone(), (vs, tree));
            Ok(SExpression::Atom(format!("defined {name}")))
        } else {
            Err("defun requires three arguments".to_string())
        }
    }

    pub fn cons(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let (Some(x), Some(xs)) = (args.next(), args.next()) {
            if let SExpression::List(mut xs) = xs.eval(s, false)? {
                xs.insert(0, x.eval(s, false)?);
                Ok(SExpression::List(xs))
            } else {
                Err("cons second argument must be a list".to_string())
            }
        } else {
            Err("cons requires two arguments".to_string())
        }
    }

    pub fn cd(mut args: Box<dyn Iterator<Item = SExpression>>, s: &mut State) -> Result<SExpression, String> {
        if let Some(e) = args.next() {
            set_current_dir(e.eval(s, false)?.ident())
                .map_err(|_| "Failed to change directory".to_string())?;
        } else {
            set_current_dir("/home/devin")
                .map_err(|_| "Failed to change directory".to_string())?;
        }

        Ok(SExpression::Atom("".to_string()))
    }
}

lazy_static! {
    pub static ref BUILTINS: HashMap<&'static str, Func> = {
        let mut m = HashMap::new();

        m.insert("+", builtin::add as Func);
        m.insert("-", builtin::sub);
        m.insert("*", builtin::mul);
        m.insert("/", builtin::div);
        m.insert("%", builtin::modulus);
        m.insert("^", builtin::pow);

        m.insert("<", builtin::lt);
        m.insert(">", builtin::gt);
        m.insert("<=", builtin::leq);
        m.insert(">=", builtin::geq);
        m.insert("=", builtin::eq);
        m.insert("if", builtin::ifs);
        m.insert("or", builtin::or);
        m.insert("and", builtin::and);
        m.insert("not", builtin::not);

        m.insert("first", builtin::first);
        m.insert("rest", builtin::rest);
        m.insert("list", builtin::list);
        m.insert("cons", builtin::cons);

        m.insert("defun", builtin::defun);
        m.insert("def", builtin::def);

        m.insert("cd", builtin::cd);
        m
    };
}

impl SExpression {
    pub fn eval(self, state: &mut State, base: bool) -> Result<SExpression, String> {
        match self {
            Self::Call(es) => {
                let mut i = es.into_iter();


                let func = i.next().unwrap();
                let args = i.collect::<Vec<_>>();

                // If func is a builtin method, run it and print result.
                if let Some(f) = BUILTINS.get(func.ident()) {
                    return f(Box::new(args.into_iter()), state);
                }

                // If func is in user defined functions then run the subs
                if let Some((vars, tree)) = state.funcs.get(func.ident()) {
                    let tree = vars.iter()
                        .zip(args.iter())
                        .fold(tree.clone(), |t, (var, sub)| {
                            t.replace(var, sub.clone())
                        });
                    return tree.eval(state, false);
                }

                let (fd_read, fd_write) = pipe().unwrap();

                // Else search path for binary, fork, and exec it with args
                let bin = state.search_path(func.ident());
                if bin.is_none() {
                    return Err(format!("command not found: {}", func))
                }

                let bin = bin.unwrap();

                let mut args = args.iter()
                    .map(|e| e.ident())
                    .map(|s| CString::new(s).unwrap())
                    .collect::<Vec<_>>();

                args.insert(0, CString::new(func.ident()).unwrap());

                // Fork, Exec
                match unsafe{fork()} {
                    Ok(ForkResult::Parent { child, .. }) => {
                        let mut out = String::new();
                        let mut buf = [0; 1024];

                        close(fd_write).unwrap();

                        if !base {
                            while let Ok(n) = read(fd_read, &mut buf) {
                                if n == 0 { break; } // EOF
                                out.push_str(std::str::from_utf8(&buf[0..n]).unwrap());
                            }

                            waitpid(child, None).unwrap();

                            let lines = out.lines()
                                .map(|s| SExpression::Atom(s.to_string()))
                                .collect::<Vec<_>>();

                            Ok(SExpression::List(lines))
                        } else {
                            waitpid(child, None).unwrap();
                            Ok(SExpression::Atom("".into()))
                        }

                    }
                    Ok(ForkResult::Child) => {
                        if !base {
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
            Self::Atom(s) => {
                if let Some(expr) = state.defs.get(&s) {
                    Ok(expr.clone())
                } else {
                    Ok(Self::Atom(s))
                }
            }
            a => Ok(a),
        }
    }

    pub fn ident(&self) -> &str {
        match self {
            SExpression::Atom(s) => s,
            _ => unimplemented!()
        }
    }

    pub fn list(&self) -> Vec<String> {
        let mut ls = Vec::new();

        match self {
            Self::Call(es) | Self::List(es) => {
                es.iter().for_each(|e| ls.push(e.ident().to_string()))
            },
            Self::Atom(s) => ls.push(s.clone())
        }

        ls
    }

    fn replace(self, from: &str, to: SExpression) -> SExpression {
        match self {
            Self::Atom(s) => {
                if s==from {
                    to
                } else {
                    Self::Atom(s)
                }
            }
            Self::Call(es) => {
                Self::Call(es.into_iter()
                           .map(|e| e.replace(from, to.clone()))
                           .collect())
            }
            a => a
        }
    }
}
