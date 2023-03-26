mod helper;
mod session;

use arrow::error::ArrowError;
use isatty::stdin_isatty;

#[tokio::main]
pub async fn main() -> Result<(), ArrowError> {
    // Authenticate
    let host = "127.0.0.1";
    let port = 4100;
    let url = format!("http://{host}:{port}");
    let mut session = session::Session::try_new(&url, "root", "").await?;

    if stdin_isatty() {
        session.handle_repl().await;
    }
    Ok(())
}
