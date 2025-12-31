use arrow::csv::WriterBuilder;
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use arrow_cast::pretty::pretty_format_batches;
use arrow_flight::{
    flight_service_client::FlightServiceClient, sql::client::FlightSqlServiceClient,
};
use futures::TryStreamExt;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use std::io::BufRead;
use tokio::time::Instant;
use tonic::transport::{Channel, Endpoint};

use crate::helper::CliHelper;

pub struct Session {
    client: FlightSqlServiceClient<Channel>,
    is_repl: bool,
    prompt: String,
    prepared: bool,
}

impl Session {
    pub async fn try_new(
        endpoint: Endpoint,
        user: &str,
        password: &str,
        is_repl: bool,
        prepared: bool,
    ) -> Result<Self, ArrowError> {
        let channel = endpoint
            .connect()
            .await
            .map_err(|err| ArrowError::IpcError(err.to_string()))?;

        if is_repl {
            println!("Welcome to Arrow CLI v{}.", env!("CARGO_PKG_VERSION"));
            println!("Connecting to {} as user {}.", endpoint.uri(), user);
            println!();
        }

        let mut client = FlightSqlServiceClient::new_from_inner(
            FlightServiceClient::new(channel).max_decoding_message_size(usize::MAX),
        );
        let _token = client.handshake(user, password).await?;

        let prompt = format!("{} :) ", endpoint.uri().host().unwrap());
        Ok(Self {
            client,
            is_repl,
            prompt,
            prepared,
        })
    }

    pub async fn handle(&mut self) {
        if self.is_repl {
            self.handle_repl().await;
        } else {
            self.handle_stdin().await;
        }
    }

    pub async fn handle_repl(&mut self) {
        let mut query = "".to_owned();
        let mut rl = Editor::<CliHelper, DefaultHistory>::new().unwrap();
        rl.set_helper(Some(CliHelper::new()));
        rl.load_history(&get_history_path()).ok();

        loop {
            match rl.readline(&self.prompt) {
                Ok(line) if line.starts_with("--") => {
                    continue;
                }
                Ok(line) => {
                    let line = line.trim_end();
                    query.push_str(&line.replace("\\\n", ""));
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
                let _ = rl.add_history_entry(query.trim_end());
                match self.handle_query(true, &query).await {
                    Ok(true) => {
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        eprintln!("handle_query err: {e}");
                    }
                }
            }
            query.clear();
        }

        println!("Bye");
        let _ = rl.save_history(&get_history_path());
    }

    pub async fn handle_stdin(&mut self) {
        let mut lines = std::io::stdin().lock().lines();
        // TODO support multi line
        while let Some(Ok(line)) = lines.next() {
            let line = line.trim_end();
            if let Err(e) = self.handle_query(false, line).await {
                eprintln!("handle_query err: {e}");
            }
        }
    }

    pub async fn handle_query(&mut self, is_repl: bool, query: &str) -> Result<bool, ArrowError> {
        if is_repl {
            if query == "exit" || query == "quit" || query == r#"\q"# {
                return Ok(true);
            }
            println!("\n{}\n", query);
        }

        let start = Instant::now();
        let flight_info = if self.prepared {
            let mut stmt = self.client.prepare(query.to_string(), None).await?;
            let info = stmt.execute().await?;
            stmt.close().await?;
            info
        } else {
            self.client.execute(query.to_string(), None).await?
        };
        let ticket_recv_duration = start.elapsed();
        let mut batches: Vec<RecordBatch> = Vec::new();

        let mut handles = Vec::with_capacity(flight_info.endpoint.len());
        for endpoint in flight_info.endpoint {
            let ticket = endpoint
                .ticket
                .as_ref()
                .ok_or_else(|| ArrowError::IpcError("Ticket is emtpy".to_string()))?
                .clone();
            let mut client = self.client.clone();
            handles.push(tokio::spawn(async move {
                let flight_data = client.do_get(ticket).await?;
                let result: Vec<RecordBatch> = flight_data.try_collect().await.map_err(|e| {
                    ArrowError::IpcError(format!("Failed to collect record batches: {e}"))
                })?;
                Ok::<Vec<RecordBatch>, ArrowError>(result)
            }));
        }

        for handle in handles {
            batches.extend(handle.await.unwrap()?);
        }
        let rows_recv_duration = start.elapsed();

        if is_repl {
            let res = pretty_format_batches(batches.as_slice())?;

            println!("{res}");
            println!();

            let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            println!(
                "{} rows in set (tickets received in {:.3} sec, rows received in {:.3} sec)",
                rows,
                ticket_recv_duration.as_secs_f64(),
                rows_recv_duration.as_secs_f64(),
            );
            println!();
        } else {
            let res = print_batches_with_sep(batches.as_slice(), b'\t')?;
            print!("{res}");
        }

        Ok(false)
    }
}

fn print_batches_with_sep(batches: &[RecordBatch], delimiter: u8) -> Result<String, ArrowError> {
    let mut bytes = vec![];
    {
        let builder = WriterBuilder::new()
            .with_header(false)
            .with_delimiter(delimiter);
        let mut writer = builder.build(&mut bytes);
        for batch in batches {
            writer.write(batch)?;
        }
    }
    let formatted = String::from_utf8(bytes).map_err(|e| ArrowError::CsvError(e.to_string()))?;
    Ok(formatted)
}

fn get_history_path() -> String {
    format!(
        "{}/.arrow_history",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    )
}
