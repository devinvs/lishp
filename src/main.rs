use lishp::parser::{SExpression, parse_file};
use lishp::state::State;
use lishp::parser::InputHelper;

fn main() {
    let mut rl = rustyline::Editor::new().unwrap();
    let h = InputHelper::default();

    rl.set_helper(Some(h));
    rl.load_history("/home/devin/.lishp_history").ok();    // load history if it exists

    let mut state = State::load();

    // Load .lishprc into state
    for expr in parse_file("/home/devin/.lishprc") {
        expr.eval(&mut state, false).unwrap();
    }

    loop {
        match rl.readline(&get_prompt()) {
            Ok(line) => {
                // If line is empty ignore
                if line.is_empty() { continue; }

                // Add to history
                rl.add_history_entry(&line).unwrap();

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

    rl.save_history("/home/devin/.fsh_history").unwrap();
}

fn get_prompt() -> String {
    format!("\x1b[35;5;1m❯\x1b[0m ")
}

