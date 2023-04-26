use lishp::parser::{SExpression, parse_file, parse_str};
use lishp::state::State;
use lishp::parser::InputHelper;

use std::env::current_dir;

use rustyline::config::Builder;
use rustyline::config::CompletionType;

fn main() {
    let config = Builder::default()
        .auto_add_history(true)
        .completion_type(CompletionType::List)
        .tab_stop(4)
        .build();

    let mut rl = rustyline::Editor::with_config(config).unwrap();
    let h = InputHelper::default();

    rl.set_helper(Some(h));
    rl.load_history("/home/devin/.lishp_history").ok();    // load history if it exists

    let mut state = State::load();

    // Load prelude into state
    for expr in parse_str(include_str!("prelude.lisp")) {
        expr.eval(&mut state, false).unwrap();
    }

    // Load .lishprc into state
    for expr in parse_file("/home/devin/.lishprc") {
        expr.eval(&mut state, false).unwrap();
    }

    let mut args = std::env::args();
    args.next();

    if let Some(s) = args.next() {
        if s == "-c" {
            let cmd = args.collect::<Vec<_>>();
            let cmd = cmd.join(" ");

            match SExpression::parse(&cmd) {
                Ok(expr) => {
                    match expr.eval(&mut state, true) {
                        Ok(e) => {
                            match e {
                                SExpression::Atom(s) if s=="" => println!(""),
                                _ => println!("{e}")
                            }
                        },
                        Err(e) => eprintln!("Error: {e}")
                    }
                }
                Err(e) => eprintln!("{e}")
            }

            return;
        }
    }

    loop {
        match rl.readline(&get_prompt()) {
            Ok(line) => {
                // If line is empty ignore
                if line.is_empty() { continue; }

                let line = state.preprocess(&line);

                // Parse and run
                match SExpression::parse(&line) {
                    Ok(expr) => {
                        match expr.eval(&mut state, true) {
                            Ok(e) => {
                                match e {
                                    SExpression::Atom(s) if s=="" => println!(""),
                                    _ => println!("{e}")
                                }
                            },
                            Err(e) => eprintln!("Error: {e}")
                        }
                    }
                    Err(e) => eprintln!("{e}")
                }
            }
            Err(_) => break
        }
    }

    rl.save_history("/home/devin/.lishp_history").unwrap();
}

fn get_cwd() -> String {
    let dir = current_dir().unwrap().to_str().unwrap().to_string();

    dir.replace("/home/devin", "~")
}

fn get_prompt() -> String {
    format!("{}\n\x1b[35;5;1m❯\x1b[0m ", get_cwd())
}

