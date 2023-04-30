use lishp::Interpreter;
use lishp::SExpression;
use lishp::completer::InputHelper;

use rustyline::config::Builder;
use rustyline::config::CompletionType;

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
        _ => run_interactive(it)
    }
}

fn run_command(it: &mut Interpreter, cmd: &str) {
    match it.eval(cmd) {
        Ok(e) => match e {
            SExpression::Atom(s) if s.is_empty() => println!(""),
            _ => println!("{e}")
        }
        Err(e) => eprintln!("Error: {e}")
    }
}

fn run_interactive(mut it: Interpreter) {
    // Ignore ctrl-c
    ctrlc::set_handler(move || {}).unwrap();

    // Setup readline completion and history
    let config = Builder::default()
        .completion_type(CompletionType::List)
        .tab_stop(4)
        .build();

    let mut rl = rustyline::Editor::with_config(config).unwrap();
    let h = InputHelper::default();

    rl.set_helper(Some(h));
    rl.load_history("/home/devin/.lishp_history").ok();    // load history if it exists

    loop {
        match rl.readline(&get_prompt(&mut it)) {
            Ok(line) => {
                // If line is empty ignore
                if line.is_empty() { continue; }
                rl.add_history_entry(&line).unwrap();
                run_command(&mut it, &line);
            }
            Err(_) => break
        }
    }

    rl.save_history("/home/devin/.lishp_history").unwrap();
}

fn get_prompt(it: &mut Interpreter) -> String {
    if let Some(e) = it.defs.get(&"lishp_prompt".chars().collect()) {
        if let Ok(s) = it.eval_expr(e.clone(), false) {
            return s.ident().iter().collect();
        }
    }

    "> ".to_string()
}

