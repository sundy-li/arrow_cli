mod handler;
mod helper;


use isatty::stdin_isatty;

fn main() -> Result<(), String> {
    // Authenticate

    if stdin_isatty() {
        let mut handler = handler::Handler {};
        handler.handle_repl();
    }
    Ok(())
}
