mod handler;
mod helper;

use clap::Parser;
use isatty::stdin_isatty;

#[derive(Debug, Parser, PartialEq)]
struct Args {
    #[clap(short = 'u', long, default_value = "root", help = "User name")]
    user: String,

    #[clap(short = 'p', long, default_value = "", help = "User password")]
    password: String,

    #[clap(
        short = 'h',
        long,
        default_value = "127.0.0.1",
        help = "Flight SQL Server host"
    )]
    host: String,
    #[clap(
        short = 'P',
        long,
        default_value_t = 4100,
        help = "Flight SQL Server port"
    )]
    port: u16,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    // Authenticate

    if stdin_isatty() {
        let mut handler = handler::Handler {};
        handler.handle_repl();
    }
    Ok(())
}
