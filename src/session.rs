use arrow_array::RecordBatch;
use arrow_cast::pretty::pretty_format_batches;
use arrow_csv::WriterBuilder;
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

use crate::{Args, helper::CliHelper};

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
                    let (batches, ticket_recv_duration, rows_recv_duration, flight_info) =
                        self.execute_query(&query).await?;
                    print_batches(
                        &batches,
                        ticket_recv_duration,
                        rows_recv_duration,
                        flight_info,
                        &self.args,
                    )?;
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
            let (batches, ticket_recv_duration, rows_recv_duration, flight_info) =
                self.execute_query(command).await?;

            print_batches(
                &batches,
                ticket_recv_duration,
                rows_recv_duration,
                flight_info,
                &self.args,
            )?;
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
                let (batches, _, _, _) = self.execute_query(line).await?;
                print_batches_with_sep(batches.as_slice(), b'\t')?;
                Ok::<_, ArrowError>(())
            }
            .await
            {
                eprintln!("handle query {line} err: {e}");
            }
        }
    }

    async fn execute_query(
        &mut self,
        query: &str,
    ) -> Result<
        (
            Vec<RecordBatch>,
            std::time::Duration,
            std::time::Duration,
            arrow_flight::FlightInfo,
        ),
        ArrowError,
    > {
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

        Ok((
            batches,
            ticket_recv_duration,
            rows_recv_duration,
            flight_info,
        ))
    }
}

fn print_batches(
    batches: &[RecordBatch],
    ticket_recv_duration: Duration,
    rows_recv_duration: Duration,
    flight_info: FlightInfo,
    args: &Args,
) -> Result<(), ArrowError> {
    let res = pretty_format_batches(batches)?;

    println!("{res}\n");

    if args.print_schema {
        let schema = flight_info.try_decode_schema()?;
        println!("{schema:#?}\n");
    }

    let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    println!(
        "{} rows in set (tickets received in {:.3} sec, rows received in {:.3} sec)\n",
        rows,
        ticket_recv_duration.as_secs_f64(),
        rows_recv_duration.as_secs_f64(),
    );
    Ok(())
}

fn print_batches_with_sep(batches: &[RecordBatch], delimiter: u8) -> Result<(), ArrowError> {
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
    print!("{formatted}");
    Ok(())
}

fn get_history_path() -> String {
    format!(
        "{}/.arrow_history",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    )
}
