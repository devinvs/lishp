use crate::Interpreter;
use crate::SExpression;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::LinkedList as List;
use std::env::set_current_dir;
use std::env::{set_var, var};
use std::fs::File;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;

use lazy_static::lazy_static;

type Func = fn(List<SExpression>, &mut Interpreter) -> Result<SExpression, String>;
type BinNum = fn(f64, f64) -> f64;
type BinCmp = fn(f64, f64) -> bool;

fn to_f64(e: SExpression) -> Result<f64, String> {
    let name = e.ident().iter().collect::<String>();

    name.parse::<f64>()
        .map_err(|_| format!("{} is not a number", &name))
}

fn fold_nums(
    args: List<SExpression>,
    init: f64,
    f: BinNum,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    let mut accum = init;

    for arg in args {
        let x = to_f64(s.eval_expr(arg, false)?)?;
        accum = f(accum, x);
    }

    let accum = accum.to_string().chars().collect();

    Ok(SExpression::Atom(accum))
}

fn bin_num(
    x: SExpression,
    y: SExpression,
    f: BinNum,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    let n = f(
        to_f64(s.eval_expr(x, false)?)?,
        to_f64(s.eval_expr(y, false)?)?,
    );

    Ok(SExpression::Atom(n.to_string().chars().collect()))
}

pub fn builtin_add(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    fold_nums(args, 0.0, |a, b| a + b, s)
}

pub fn builtin_sub(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_num(x, y, |a, b| a - b, s)
    } else {
        Err("sub requires two arguments".to_string())
    }
}

pub fn builtin_mul(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    fold_nums(args, 1.0, |a, b| a * b, s)
}

pub fn builtin_div(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_num(x, y, |a, b| a / b, s)
    } else {
        Err("div requires two arguments".to_string())
    }
}

pub fn builtin_mod(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_num(x, y, |a, b| a % b, s)
    } else {
        Err("mod requires two arguments".to_string())
    }
}

pub fn builtin_pow(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_num(x, y, |a, b| a.powf(b), s)
    } else {
        Err("pow requires two arguments".to_string())
    }
}

fn bin_cmp(
    x: SExpression,
    y: SExpression,
    f: BinCmp,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    let b = f(
        to_f64(s.eval_expr(x, false)?)?,
        to_f64(s.eval_expr(y, false)?)?,
    );

    Ok(SExpression::Atom(b.to_string().chars().collect()))
}

pub fn builtin_not(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let Some(e) = args.pop_front() {
        match s.eval_expr(e, false)? {
            SExpression::Atom(s) => match s.iter().collect::<String>().as_str() {
                "true" => Ok(SExpression::Atom("false".to_string().chars().collect())),
                "false" => Ok(SExpression::Atom("true".to_string().chars().collect())),
                _ => Err("not expects a boolean argument".to_string()),
            },
            _ => Err("not expects a boolean argument".to_string()),
        }
    } else {
        Err("not requires one argument".to_string())
    }
}

pub fn builtin_or(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    let mut accum = false;

    for arg in args {
        let x = s.eval_expr(arg, false)?;
        accum = accum || x.ident().iter().collect::<String>() == "true";
    }

    Ok(SExpression::Atom(accum.to_string().chars().collect()))
}

pub fn builtin_and(args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    let mut accum = true;

    for arg in args {
        let x = s.eval_expr(arg, false)?;
        accum = accum && x.ident().iter().collect::<String>() == "true";
    }

    Ok(SExpression::Atom(accum.to_string().chars().collect()))
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
        bin_cmp(
            x,
            y,
            |a, b| a.partial_cmp(&b).unwrap() == Ordering::Greater,
            s,
        )
    } else {
        Err("gt requires two arguments".to_string())
    }
}

pub fn builtin_leq(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_cmp(
            x,
            y,
            |a, b| a == b || a.partial_cmp(&b).unwrap() == Ordering::Less,
            s,
        )
    } else {
        Err("leq requires two arguments".to_string())
    }
}

pub fn builtin_geq(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(x), Some(y)) = (args.pop_front(), args.pop_front()) {
        bin_cmp(
            x,
            y,
            |a, b| a == b || a.partial_cmp(&b).unwrap() == Ordering::Greater,
            s,
        )
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
            (SExpression::Atom(s), SExpression::List(l))
            | (SExpression::List(l), SExpression::Atom(s)) => {
                if s.is_empty() && l.is_empty() {
                    return Ok(SExpression::Atom("true".to_string().chars().collect()));
                }
            }
            _ => {}
        }

        Ok(SExpression::Atom((x == y).to_string().chars().collect()))
    } else {
        Err("eq requires two arguments".to_string())
    }
}

pub fn builtin_if(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let (Some(cond), Some(t), Some(f)) = (args.pop_front(), args.pop_front(), args.pop_front()) {
        if s.eval_expr(cond, false)?.ident().iter().collect::<String>() == "true" {
            s.eval_expr(t, false)
        } else {
            s.eval_expr(f, false)
        }
    } else {
        Err("if requires three arguments".to_string())
    }
}

pub fn builtin_first(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let Some(e) = args.pop_front() {
        match s.eval_expr(e, false)? {
            SExpression::List(mut es) => {
                if let Some(e) = es.pop_front() {
                    Ok(e)
                } else {
                    Err("tried to call first on empty list".to_string())
                }
            }
            SExpression::Atom(mut s) => {
                if let Some(c) = s.pop_front() {
                    let mut l = List::new();
                    l.push_front(c);
                    Ok(SExpression::Atom(l))
                } else {
                    Err("tried to call first on empty string".to_string())
                }
            }
            _ => unreachable!(),
        }
    } else {
        Err("first requires one argument".to_string())
    }
}

pub fn builtin_rest(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let Some(e) = args.pop_front() {
        match s.eval_expr(e, false)? {
            SExpression::List(mut es) => {
                es.pop_front();

                Ok(SExpression::List(es))
            }
            SExpression::Atom(mut s) => {
                s.pop_front();
                Ok(SExpression::Atom(s))
            }
            _ => unreachable!(),
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

pub fn builtin_def(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(SExpression::Atom(name)), Some(val)) = (args.pop_front(), args.pop_front()) {
        s.defs.insert(name.clone(), val);
        Ok(SExpression::Atom(
            format!("defined {}", name.iter().collect::<String>())
                .chars()
                .collect(),
        ))
    } else {
        Err("def requires two arguments".to_string())
    }
}

pub fn builtin_defun(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(SExpression::Atom(name)), Some(vars), Some(mut tree)) =
        (args.pop_front(), args.pop_front(), args.pop_front())
    {
        let mut vs = Vec::new();

        // rename variables to be unique...
        for var in vars.list() {
            let var_name = var.iter().collect::<String>();
            let new = format!("#{}#{}#", var_name, s.id)
                .chars()
                .collect::<List<char>>();
            vs.push(new.clone());
            tree = tree.replace(&var, SExpression::Atom(new));
            s.id += 1;
        }

        s.funcs.insert(name.clone(), (vs, tree));
        Ok(SExpression::Atom(
            format!("defined {}", name.iter().collect::<String>())
                .chars()
                .collect(),
        ))
    } else {
        Err("defun requires three arguments".to_string())
    }
}

pub fn builtin_alias(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let Some(SExpression::Atom(from)) = args.pop_front() {
        let to = args.into_iter().map(|e| e.ident()).collect::<List<_>>();

        s.aliases.insert(from.clone(), to.clone());

        Ok(SExpression::Atom(
            format!("created alias").chars().collect(),
        ))
    } else {
        Err("alias requires two arguments".to_string())
    }
}

pub fn builtin_cons(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(x), Some(xs)) = (args.pop_front(), args.pop_front()) {
        let x = s.eval_expr(x, false)?;
        let xs = s.eval_expr(xs, false)?;

        match (x, xs) {
            (SExpression::Atom(c), SExpression::List(xs)) if c.len() == 1 && xs.len() == 0 => {
                Ok(SExpression::Atom(c))
            }
            (SExpression::Atom(mut c), SExpression::Atom(mut s)) if c.len() == 1 => {
                c.append(&mut s);
                Ok(SExpression::Atom(c))
            }
            (x, SExpression::List(mut xs)) => {
                xs.push_front(s.eval_expr(x, false)?);
                Ok(SExpression::List(xs))
            }
            (_, _) => Err("cons second argument must be list-like".to_string()),
        }
    } else {
        Err("cons requires two arguments".to_string())
    }
}

pub fn builtin_cd(mut args: List<SExpression>, s: &mut Interpreter) -> Result<SExpression, String> {
    if let Some(e) = args.pop_front() {
        set_current_dir(s.eval_expr(e, false)?.ident().iter().collect::<String>())
            .map_err(|e| format!("Failed to change directory: {e}"))?;
    } else {
        set_current_dir("/home/devin").map_err(|e| format!("Failed to change directory: {e}"))?;
    }

    Ok(SExpression::Atom(List::new()))
}

pub fn builtin_exit(_: List<SExpression>, _: &mut Interpreter) -> Result<SExpression, String> {
    std::process::exit(0)
}

pub fn builtin_let(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    // let statement looks like: (let ((var val)...) expr)
    if let (Some(SExpression::Call(mut pairs)), Some(mut e)) = (args.pop_front(), args.pop_front())
    {
        while let (Some(SExpression::Atom(from)), Some(to)) = (pairs.pop_front(), pairs.pop_front())
        {
            let to = s.eval_expr(to, false)?;
            e = e.replace(&from, to);
        }
        return s.eval_expr(e, false);
    }

    Err("let requires two arguments".to_string())
}

pub fn builtin_getenv(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let Some(a) = args.pop_front() {
        let v: String = s.eval_expr(a, false)?.ident().into_iter().collect();
        let val: List<char> = var(&v)
            .map_err(|_| "env var not found".to_string())?
            .chars()
            .collect();
        Ok(SExpression::Atom(val))
    } else {
        Err("getenv requires one argument".to_string())
    }
}

pub fn builtin_export(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(a), Some(b)) = (args.pop_front(), args.pop_front()) {
        let var: String = s.eval_expr(a, false)?.ident().into_iter().collect();
        let val: String = s.eval_expr(b, false)?.ident().into_iter().collect();
        set_var(&var, &val);

        return Ok(SExpression::Atom(List::new()));
    }

    Err("export requires two arguments".to_string())
}

pub fn builtin_file_write(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(content), Some(file)) = (args.pop_front(), args.pop_front()) {
        let content: String = s.eval_expr(content, false)?.ident().into_iter().collect();
        let file: String = s.eval_expr(file, false)?.ident().into_iter().collect();

        let f = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file)
            .map_err(|e| format!("write: {e}"))?;
        let mut writer = BufWriter::new(f);

        writer
            .write_all(content.as_bytes())
            .map_err(|e| format!("write: {e}"))?;

        return Ok(SExpression::Atom(List::new()));
    }

    Err("write requires two arguments".to_string())
}

pub fn builtin_file_append(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let (Some(content), Some(file)) = (args.pop_front(), args.pop_front()) {
        let content: String = s.eval_expr(content, false)?.ident().into_iter().collect();
        let file: String = s.eval_expr(file, false)?.ident().into_iter().collect();

        let f = File::options()
            .append(true)
            .create(true)
            .open(file)
            .map_err(|e| format!("append: {e}"))?;

        let mut writer = BufWriter::new(f);

        writer
            .write_all(content.as_bytes())
            .map_err(|e| format!("append: {e}"))?;

        return Ok(SExpression::Atom(List::new()));
    }

    Err("append requires two arguments".to_string())
}

pub fn builtin_file_read(
    mut args: List<SExpression>,
    s: &mut Interpreter,
) -> Result<SExpression, String> {
    if let Some(file) = args.pop_front() {
        let file: String = s.eval_expr(file, false)?.ident().into_iter().collect();
        let mut f = File::open(file).map_err(|e| format!("read: {e}"))?;
        let mut out = String::new();
        f.read_to_string(&mut out)
            .map_err(|e| format!("read: {e}"))?;

        return Ok(SExpression::Atom(out.chars().collect()));
    }

    Err("read requires one argument".to_string())
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

        m.insert("export", builtin_export);
        m.insert("getenv", builtin_getenv);

        m.insert("cd", builtin_cd);
        m.insert("exit", builtin_exit);

        m.insert("write", builtin_file_write);
        m.insert("append", builtin_file_append);
        m.insert("read", builtin_file_read);
        m
    };
}
