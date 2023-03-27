mod helper;
mod session;

use arrow::error::ArrowError;

use isatty::stdin_isatty;

use clap::Parser;

#[derive(Debug, Parser, PartialEq)]
#[command(disable_help_flag = true)]
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

    #[clap(long, help = "Print help information")]
    help: bool,
}

#[tokio::main]
pub async fn main() -> Result<(), ArrowError> {
    let args = Args::parse();
    if args.help {
        print_usage();
        return Ok(());
    }

    // Authenticate
    let url = format!("http://{}:{}", args.host, args.port);
    let mut session = session::Session::try_new(&url, &args.user, &args.password).await?;

    if stdin_isatty() {
        session.handle_repl().await;
    } else {
        session.handle_stdin().await;
    }
    Ok(())
}

fn print_usage() {
    let msg =
        r#"Usage: arrow_cli <--user <USER>|--password <PASSWORD>|--host <HOST>|--port <PORT>>"#;
    println!("{}", msg);
}
