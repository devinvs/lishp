use lishp::History;
use lishp::Input;
use lishp::Interpreter;
use lishp::SExpression;

fn main() {
    let mut it = Interpreter::load();

    // If -c flag is used run the command from args and then exit, else start interpreter
    let mut args = std::env::args();
    match args.nth(1).as_ref().map(|s| s.as_str()) {
        Some("-c") => {
            let cmd = args.collect::<Vec<_>>();
            let cmd = cmd.join(" ");
            run_command(&mut it, &cmd);
        }
        _ => run_interactive(it),
    }
}

fn run_command(it: &mut Interpreter, cmd: &str) {
    match it.eval(cmd) {
        Ok(e) => match e {
            SExpression::Atom(s) if s.is_empty() => println!(""),
            _ => println!("{e}"),
        },
        Err(e) => eprintln!("Error: {e}"),
    }
}

fn run_interactive(mut it: Interpreter) {
    // Ignore ctrl-c
    ctrlc::set_handler(move || {}).unwrap();

    let input = Input::new();
    let mut history = History::Nil;

    loop {
        let prompt = get_prompt(&mut it);
        match input.readline(&prompt, history.clone()) {
            Ok(s) => {
                match history.clone() {
                    History::Nil => {
                        history = History::Cons(s.clone(), Box::new(history));
                    }
                    History::Cons(a, _) if a != s => {
                        history = History::Cons(s.clone(), Box::new(history));
                    }
                    _ => {}
                }

                run_command(&mut it, &s);
            }
            _ => {}
        }
    }
}

fn get_prompt(it: &mut Interpreter) -> String {
    if let Some(e) = it.defs.get(&"lishp_prompt".chars().collect()) {
        if let Ok(s) = it.eval_expr(e.clone(), false) {
            return s.ident().iter().collect();
        }
    }

    "> ".to_string()
}
