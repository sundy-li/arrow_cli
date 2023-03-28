use arrow::csv::WriterBuilder;
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use arrow_cast::pretty::pretty_format_batches;
use arrow_flight::sql::client::FlightSqlServiceClient;
use arrow_flight::utils::flight_data_to_batches;
use arrow_flight::FlightData;
use futures::TryStreamExt;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::io::BufRead;
use tokio::time::Instant;
use tonic::transport::Endpoint;

use crate::helper::CliHelper;

pub struct Session {
    client: FlightSqlServiceClient,
    is_repl: bool,
    prompt: String,
}

impl Session {
    pub async fn try_new(
        endpoint: Endpoint,
        user: &str,
        password: &str,
        is_repl: bool,
    ) -> Result<Self, ArrowError> {
        let channel = endpoint
            .connect()
            .await
            .map_err(|err| ArrowError::IoError(err.to_string()))?;

        if is_repl {
            println!("Welcome to Arrow CLI.");
            println!("Connecting to {} as user {}.", endpoint.uri(), user);
            println!();
        }
        let mut client = FlightSqlServiceClient::new(channel);
        let _token = client.handshake(user, password).await.unwrap();

        let prompt = format!("{} :) ", endpoint.uri().host().unwrap());
        Ok(Self {
            client,
            is_repl,
            prompt,
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
            if query == "exit" || query == "quit" {
                return Ok(true);
            }
            println!("\n{}\n", query);
        }

        let start = Instant::now();
        let mut stmt = self.client.prepare(query.to_string()).await?;
        let flight_info = stmt.execute().await?;
        let ticket = flight_info.endpoint[0]
            .ticket
            .as_ref()
            .ok_or_else(|| ArrowError::IoError("Ticket is emtpy".to_string()))?;

        let flight_data = self.client.do_get(ticket.clone()).await?;
        let flight_data: Vec<FlightData> = flight_data.try_collect().await.unwrap();

        let batches = flight_data_to_batches(&flight_data)?;
        if is_repl {
            let res = pretty_format_batches(batches.as_slice())?;

            println!("{res}");
            println!();

            let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            println!(
                "{} rows in set ({:.3} sec)",
                rows,
                start.elapsed().as_secs_f64()
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
            .has_headers(false)
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
