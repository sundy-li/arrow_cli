mod helper;
mod session;

use arrow::error::ArrowError;

use isatty::stdin_isatty;

use clap::Parser;

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

#[tokio::main]
pub async fn main() -> Result<(), ArrowError> {
    let args = Args::parse();
    // Authenticate
    let url = format!("http://{}:{}", args.host, args.port);
    let mut session = session::Session::try_new(&url, "root", "").await?;

    if stdin_isatty() {
        session.handle_repl().await;
    }
    Ok(())
}
