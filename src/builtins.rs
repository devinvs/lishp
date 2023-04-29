use std::cmp::Ordering;
use std::env::set_current_dir;
use std::collections::HashMap;
use std::collections::LinkedList as List;
use crate::SExpression;
use crate::Interpreter;

use lazy_static::lazy_static;

type Func = fn(List<SExpression>, &mut Interpreter) -> Result<SExpression, String>;
type BinNum = fn(f64, f64) -> f64;
type BinCmp = fn(f64, f64) -> bool;

fn to_f64(e: &SExpression) -> Result<f64, String> {
    e.ident().parse::<f64>()
        .map_err(|_| format!("{e} is not a number"))
}

fn fold_nums(args: List<SExpression>, init: f64, f: BinNum, s: &mut Interpreter) -> Result<SExpression, String> {
    let mut accum = init;

    for arg in args {
        let x = to_f64(&s.eval_expr(arg, false)?)?;
        accum = f(accum, x);
    }

    Ok(SExpression::Atom(accum.to_string()))
}

fn bin_num(x: SExpression, y: SExpression, f: BinNum, s: &mut Interpreter) -> Result<SExpression, String> {
    Ok(SExpression::Atom(f(
        to_f64(&s.eval_expr(x, false)?)?,
        to_f64(&s.eval_expr(y, false)?)?).to_string()))
}

pub fn builtin_add(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    fold_nums(args, 0.0, |a, b| a+b, s)
}

pub fn builtin_sub(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_num(x, y, |a, b| a - b, s)
    } else {
        Err("sub requires two arguments".to_string())
    }
}

pub fn builtin_mul(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    fold_nums(args, 1.0, |a, b| a*b, s)
}

pub fn builtin_div(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_num(x, y, |a, b| a / b, s)
    } else {
        Err("div requires two arguments".to_string())
    }
}

pub fn builtin_mod(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_num(x, y, |a, b| a % b, s)
    } else {
        Err("mod requires two arguments".to_string())
    }
}

pub fn builtin_pow(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_num(x, y, |a, b| a.powf(b), s)
    } else {
        Err("pow requires two arguments".to_string())
    }
}

fn bin_cmp(x: SExpression, y: SExpression, f: BinCmp, s: &mut Interpreter) -> Result<SExpression, String> {
    Ok(SExpression::Atom(f(
            to_f64(&s.eval_expr(x, false)?)?,
            to_f64(&s.eval_expr(y, false)?)?).to_string()))
}

pub fn builtin_not(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let Some(e) = args.pop_front() {
        match s.eval_expr(e, false)? {
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

pub fn builtin_or(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    let mut accum = false;

    for arg in args {
        let x = s.eval_expr(arg, false)?;
        accum = accum || x.ident() == "true";
    }

    Ok(SExpression::Atom(accum.to_string()))
}

pub fn builtin_and(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    let mut accum = true;

    for arg in args {
        let x = s.eval_expr(arg, false)?;
        accum = accum && x.ident() == "true";
    }

    Ok(SExpression::Atom(accum.to_string()))
}

pub fn builtin_lt(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_cmp(x, y, |a, b| a.partial_cmp(&b).unwrap() == Ordering::Less, s)
    } else {
        Err("lt requires two arguments".to_string())
    }
}

pub fn builtin_gt(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_cmp(x, y, |a, b| a.partial_cmp(&b).unwrap() == Ordering::Greater, s)
    } else {
        Err("gt requires two arguments".to_string())
    }
}

pub fn builtin_leq(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_cmp(x, y, |a, b| a==b || a.partial_cmp(&b).unwrap() == Ordering::Less, s)
    } else {
        Err("leq requires two arguments".to_string())
    }
}

pub fn builtin_geq(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_cmp(x, y, |a, b| a==b || a.partial_cmp(&b).unwrap() == Ordering::Greater, s)
    } else {
        Err("geq requires two arguments".to_string())
    }
}

pub fn builtin_eq(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        let x = s.eval_expr(x, false)?;
        let y = s.eval_expr(y, false)?;

        // Special case where "" should equal '()
        
        match (&x, &y) {
            (SExpression::Atom(s), SExpression::List(l)) | (SExpression::List(l), SExpression::Atom(s)) => {
                if s=="" && l.is_empty() { return Ok(SExpression::Atom("true".to_string())) }
            }
            _ => {}
        }

        Ok(SExpression::Atom((x==y).to_string()))

    } else {
        Err("eq requires two arguments".to_string())
    }
}

pub fn builtin_if(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(cond), Some(t), Some(f)) = (args.pop_front(), args.pop_front(), args.pop_front()) {
        if s.eval_expr(cond, false)?.ident() == "true" {
            s.eval_expr(t, false)
        } else {
            s.eval_expr(f, false)
        }
    } else {
        Err("if requires three arguments".to_string())
    }
}

pub fn builtin_first(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let Some(e) = args.pop_front() {
        match s.eval_expr(e, false)? {
            SExpression::List(mut es) => {
                if let Some(e) = es.pop_front() {
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

pub fn builtin_rest(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let Some(e) = args.pop_front() {
        match s.eval_expr(e, false)? {
            SExpression::List(mut es) => {
                es.pop_front();

                Ok(SExpression::List(es))
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

pub fn builtin_list(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    let mut l = List::new();

    for arg in args {
        l.push_back(s.eval_expr(arg, false)?);
    }

    Ok(SExpression::List(l))
}

pub fn builtin_def(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(SExpression::Atom(name)), Some(val)) = (args.pop_front(), args.pop_front()) {
        s.defs.insert(name.clone(), val);
        Ok(SExpression::Atom(format!("defined {name}")))
    } else {
        Err("def requires two arguments".to_string())
    }
}

pub fn builtin_defun(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(SExpression::Atom(name)), Some(vars), Some(mut tree)) = (args.pop_front(), args.pop_front(), args.pop_front()) {
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

pub fn builtin_alias(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let Some(SExpression::Atom(from)) = args.pop_front() {
        let to = args.iter().map(|e| e.ident().to_string()).collect::<Vec<_>>();
        let to = to.join(" ");

        s.aliases.insert(from.clone(), to.clone());

        Ok(SExpression::Atom(format!("aliased {from} to {to}")))
    } else {
        Err("alias requires two arguments".to_string())
    }
}

pub fn builtin_cons(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(x), Some(xs)) = (args.pop_front(), args.pop_front()) {
        let x = s.eval_expr(x, false)?;
        let xs = s.eval_expr(xs, false)?;

        match (x, xs) {
            (SExpression::Atom(c), SExpression::List(xs)) if c.chars().count() == 1 && xs.len() == 0 => {
                Ok(SExpression::Atom(c))
            }
            (SExpression::Atom(mut c), SExpression::Atom(s)) if c.chars().count() == 1 => {
                c.push_str(&s);
                Ok(SExpression::Atom(c))
            }
            (x, SExpression::List(mut xs)) => {
                xs.push_front(s.eval_expr(x, false)?);
                Ok(SExpression::List(xs))
            }
            (x, xs) => {
                eprintln!("{x} vs {xs}");
                Err("cons second argument must be list-like".to_string())
            }
        }
    } else {
        Err("cons requires two arguments".to_string())
    }
}

pub fn builtin_cd(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let Some(e) = args.pop_front() {
        set_current_dir(s.eval_expr(e, false)?.ident())
            .map_err(|e| format!("Failed to change directory: {e}"))?;
    } else {
        set_current_dir("/home/devin")
            .map_err(|e| format!("Failed to change directory: {e}"))?;
    }

    Ok(SExpression::Atom("".to_string()))
}

pub fn builtin_exit(_: List<SExpression>, _: &mut Interpreter) -> Result<SExpression, String> {
    std::process::exit(0)
}

pub fn builtin_let(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    // let statement looks like: (let ((var val)...) expr)
    if let (Some(SExpression::Call(mut pairs)), Some(mut e)) = (args.pop_front(), args.pop_front()) {
        while let (Some(SExpression::Atom(from)), Some(to)) = (pairs.pop_front(), pairs.pop_front()) {
            let to = s.eval_expr(to, false)?;
            e = e.replace(&from, to);
        }
        return s.eval_expr(e, false);
    }
    
    Err("let requires two arguments".to_string())
}

lazy_static! {
    pub static ref BUILTINS: HashMap<&'static str, Func> = {
        let mut m = HashMap::new();

        m.insert("+", builtin_add as Func);
        m.insert("-", builtin_sub);
        m.insert("*", builtin_mul);
        m.insert("/", builtin_div);
        m.insert("%", builtin_mod);
        m.insert("^", builtin_pow);

        m.insert("<", builtin_lt);
        m.insert(">", builtin_gt);
        m.insert("<=", builtin_leq);
        m.insert(">=", builtin_geq);
        m.insert("=", builtin_eq);
        m.insert("if", builtin_if);
        m.insert("or", builtin_or);
        m.insert("and", builtin_and);
        m.insert("not", builtin_not);

        m.insert("first", builtin_first);
        m.insert("rest", builtin_rest);
        m.insert("list", builtin_list);
        m.insert("cons", builtin_cons);

        m.insert("defun", builtin_defun);
        m.insert("def", builtin_def);
        m.insert("alias", builtin_alias);
        m.insert("let", builtin_let);

        m.insert("cd", builtin_cd);
        m.insert("exit", builtin_exit);
        m
    };
}
