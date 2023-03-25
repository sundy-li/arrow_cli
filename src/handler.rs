
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;

use crate::helper::CliHelper;

pub(crate) struct Handler {}

const DEFAULT_PROMPT: &str = "arrow_cli :) ";
impl Handler {
    pub fn handle_repl(&mut self) {
        let mut query = "".to_owned();
        let mut rl = Editor::<CliHelper, DefaultHistory>::new().unwrap();
        rl.set_helper(Some(CliHelper::new()));
        rl.load_history(".history").ok();

        loop {
            match rl.readline(DEFAULT_PROMPT) {
                Ok(line) if line.starts_with("--") => {
                    continue;
                }
                Ok(line) => {
                    let line = line.trim_end();
                    if line.ends_with('\\') {
                        query.push_str(line[..line.len() - 1].trim_end());
                        continue;
                    } else {
                        query.push_str(line);
                    }
                }
                Err(e) => match e {
                    ReadlineError::Io(err) => {
                        eprintln!("io err: {err}");
                    }
                    ReadlineError::Interrupted => {
                        println!("^C");
                    }
                    ReadlineError::Eof => {
                        break;
                    }
                    _ => {}
                },
            }
            if !query.is_empty() {
                match self.handler_query(&query) {
                    Ok(true) => {
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        println!("handler_query err: {e}");
                    }
                }
            }
            query.clear();
        }
        println!("Bye");
    }

    fn handler_query(&mut self, query: &str) -> Result<bool, String> {
        println!("got: {}", query);
        if query == "exit" {
            return Ok(true);
        }
        Ok(false)
    }
}
