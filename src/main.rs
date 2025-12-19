mod helper;
mod session;

use std::time::Duration;

use arrow::error::ArrowError;
use atty::Stream;

use clap::Parser;
use tonic::transport::{ClientTlsConfig, Endpoint};

#[derive(Debug, Parser, PartialEq)]
struct Args {
    #[clap(short = 'u', long, default_value = "root", help = "User name")]
    user: String,

    #[clap(short = 'p', long, default_value = "", help = "User password")]
    password: String,

    #[clap(long, default_value = "127.0.0.1", help = "Flight SQL Server host")]
    host: String,
    #[clap(
        short = 'P',
        long,
        default_value_t = 4100,
        help = "Flight SQL Server port"
    )]
    port: u16,

    #[clap(long)]
    tls: bool,

    #[clap(long, default_value = "180", help = "Request timeout in seconds")]
    timeout: u64,

    #[clap(
        long,
        default_value = "false",
        help = "Execute query using prepared statement"
    )]
    prepared: bool,
}

#[tokio::main]
pub async fn main() -> Result<(), ArrowError> {
    let args = Args::parse();

    let protocol = if args.tls { "https" } else { "http" };
    // Authenticate
    let url = format!("{protocol}://{}:{}", args.host, args.port);
    let endpoint = endpoint(&args, url)?;
    let is_repl = atty::is(Stream::Stdin);
    let mut session =
        session::Session::try_new(endpoint, &args.user, &args.password, is_repl, args.prepared)
            .await?;

    session.handle().await;
    Ok(())
}

fn endpoint(args: &Args, addr: String) -> Result<Endpoint, ArrowError> {
    let mut endpoint = Endpoint::new(addr)
        .map_err(|_| ArrowError::IpcError("Cannot create endpoint".to_string()))?
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(args.timeout))
        .tcp_nodelay(true) // Disable Nagle's Algorithm since we don't want packets to wait
        .tcp_keepalive(Some(Duration::from_secs(3600)))
        .http2_keep_alive_interval(Duration::from_secs(300))
        .keep_alive_timeout(Duration::from_secs(20))
        .keep_alive_while_idle(true);

    if args.tls {
        let tls_config = ClientTlsConfig::new();
        endpoint = endpoint
            .tls_config(tls_config)
            .map_err(|_| ArrowError::IpcError("Cannot create TLS endpoint".to_string()))?;
    }

    Ok(endpoint)
}
