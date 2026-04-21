use arrow_array::RecordBatch;
use arrow_flight::{
    FlightInfo, flight_service_client::FlightServiceClient, sql::client::FlightSqlServiceClient,
};
use arrow_schema::ArrowError;
use futures::TryStreamExt;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use std::{io::BufRead, time::Duration};
use tokio::time::Instant;
use tonic::transport::{Channel, Endpoint};

use crate::{
    Args,
    helper::CliHelper,
    output::{self, Output},
};

pub struct Session {
    client: FlightSqlServiceClient<Channel>,
    is_repl: bool,
    prompt: String,
    args: Args,
}

impl Session {
    pub async fn try_new(
        endpoint: Endpoint,
        is_repl: bool,
        args: Args,
    ) -> Result<Self, ArrowError> {
        let channel = endpoint
            .connect()
            .await
            .map_err(|err| ArrowError::IpcError(err.to_string()))?;

        if is_repl {
            println!("Welcome to Arrow CLI v{}.", env!("CARGO_PKG_VERSION"));
            println!("Connecting to {} as user {}.", endpoint.uri(), args.user);
            println!();
        }

        let mut client = FlightSqlServiceClient::new_from_inner(
            FlightServiceClient::new(channel).max_decoding_message_size(usize::MAX),
        );
        let _token = client.handshake(&args.user, &args.password).await?;

        let prompt = format!("{} :) ", endpoint.uri().host().unwrap());
        Ok(Self {
            client,
            is_repl,
            prompt,
            args,
        })
    }

    pub async fn handle(&mut self) {
        if self.is_repl {
            self.handle_repl().await;
        } else if self.args.command.is_some() {
            let command = self.args.command.clone().unwrap();
            self.handle_command(&command).await;
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

                if query == "exit" || query == "quit" || query == r#"\q"# {
                    break;
                }

                println!("\n{}\n", query);

                if let Err(e) = async {
                    let result = self.execute_query(&query).await?;
                    print_query_result(&result, &self.args)?;
                    Ok::<_, ArrowError>(())
                }
                .await
                {
                    eprintln!("handle query err: {e}");
                }
            }
            query.clear();
        }

        println!("Bye");
        let _ = rl.save_history(&get_history_path());
    }

    pub async fn handle_command(&mut self, command: &str) {
        if let Err(e) = async {
            let result = self.execute_query(command).await?;
            print_query_result(&result, &self.args)?;
            Ok::<_, ArrowError>(())
        }
        .await
        {
            eprintln!("handle command {command} err: {e}");
        }
    }

    pub async fn handle_stdin(&mut self) {
        let mut lines = std::io::stdin().lock().lines();
        // TODO support multi line
        while let Some(Ok(line)) = lines.next() {
            let line = line.trim_end();
            if let Err(e) = async {
                let result = self.execute_query(line).await?;
                print_query_result(&result, &self.args)?;
                Ok::<_, ArrowError>(())
            }
            .await
            {
                eprintln!("handle query {line} err: {e}");
            }
        }
    }

    async fn execute_query(&mut self, query: &str) -> Result<QueryResult, ArrowError> {
        let start = Instant::now();
        let flight_info = if self.args.prepared {
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
        for endpoint in flight_info.endpoint.iter() {
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

        Ok(QueryResult {
            batches,
            ticket_recv_duration,
            rows_recv_duration,
            flight_info,
        })
    }
}

struct QueryResult {
    batches: Vec<RecordBatch>,
    ticket_recv_duration: Duration,
    rows_recv_duration: Duration,
    flight_info: FlightInfo,
}

fn print_query_result(result: &QueryResult, args: &Args) -> Result<(), ArrowError> {
    output::print_batches(&result.batches, args.output)?;

    if args.print_schema {
        let schema = result.flight_info.clone().try_decode_schema()?;
        println!("{schema:#?}\n");
    }

    if args.output == Output::Table {
        let rows: usize = result.batches.iter().map(|b| b.num_rows()).sum();
        println!(
            "{} rows in set (tickets received in {:.3} sec, rows received in {:.3} sec)\n",
            rows,
            result.ticket_recv_duration.as_secs_f64(),
            result.rows_recv_duration.as_secs_f64(),
        );
    }

    Ok(())
}

fn get_history_path() -> String {
    format!(
        "{}/.arrow_history",
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string())
    )
}
