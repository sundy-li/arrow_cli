use std::time::Duration;

use arrow::error::ArrowError;
use arrow_cast::pretty::pretty_format_batches;
use arrow_flight::sql::client::FlightSqlServiceClient;
use arrow_flight::utils::flight_data_to_batches;
use arrow_flight::{FlightClient, FlightData, FlightDescriptor};
use futures::TryStreamExt;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use tonic::transport::{Certificate, Endpoint};

use crate::helper::CliHelper;

pub struct Session {
    client: FlightSqlServiceClient,
}

const DEFAULT_PROMPT: &str = "arrow_cli :) ";
impl Session {
    pub async fn try_new(url: &str, user: &str, password: &str) -> Result<Self, ArrowError> {
        let endpoint = endpoint(String::from(url))?;
        let channel = endpoint
            .connect()
            .await
            .map_err(|err| ArrowError::IoError(err.to_string()))?;
        let mut client = FlightSqlServiceClient::new(channel);
        let token = client.handshake(user, password).await.unwrap();
        println!("got {:?}", token);

        Ok(Self { client })
    }

    pub async fn handle_repl(&mut self) {
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
                match self.handle_query(&query).await {
                    Ok(true) => {
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        println!("handle_query err: {e}");
                    }
                }
            }
            query.clear();
        }
        println!("Bye");
    }

    async fn handle_query(&mut self, query: &str) -> Result<bool, String> {
        if query == "exit" || query == "quit" {
            return Ok(true);
        }

        let mut stmt = self.client.prepare(query.to_string()).await.unwrap();
        let flight_info = stmt.execute().await.unwrap();
        let ticket = flight_info.endpoint[0].ticket.as_ref().unwrap().clone();
        let flight_data = self.client.do_get(ticket).await.unwrap();
        let flight_data: Vec<FlightData> = flight_data.try_collect().await.unwrap();

        let batches = flight_data_to_batches(&flight_data).unwrap();

        let res = pretty_format_batches(batches.as_slice()).unwrap();
        println!("{res}");
        Ok(false)
    }
}

fn endpoint(addr: String) -> Result<Endpoint, ArrowError> {
    let endpoint = Endpoint::new(addr)
        .map_err(|_| ArrowError::IoError("Cannot create endpoint".to_string()))?
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(20))
        .tcp_nodelay(true) // Disable Nagle's Algorithm since we don't want packets to wait
        .tcp_keepalive(Option::Some(Duration::from_secs(3600)))
        .http2_keep_alive_interval(Duration::from_secs(300))
        .keep_alive_timeout(Duration::from_secs(20))
        .keep_alive_while_idle(true);

    Ok(endpoint)
}
