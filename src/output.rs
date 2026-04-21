use arrow_array::RecordBatch;
use arrow_cast::pretty::pretty_format_batches;
use arrow_csv::WriterBuilder as CsvWriterBuilder;
use arrow_json::LineDelimitedWriter;
use arrow_schema::ArrowError;
use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Output {
    Table,
    Json,
    Csv,
    Tsv,
    Psv,
}

pub fn print_batches(batches: &[RecordBatch], output: Output) -> Result<(), ArrowError> {
    let formatted = format_batches(batches, output)?;
    print!("{formatted}");
    Ok(())
}

pub fn format_batches(batches: &[RecordBatch], output: Output) -> Result<String, ArrowError> {
    match output {
        Output::Table => format_table(batches),
        Output::Json => format_json(batches),
        Output::Csv => format_delimited(batches, b','),
        Output::Tsv => format_delimited(batches, b'\t'),
        Output::Psv => format_delimited(batches, b'|'),
    }
}

fn format_table(batches: &[RecordBatch]) -> Result<String, ArrowError> {
    Ok(format!("{}\n", pretty_format_batches(batches)?))
}

fn format_json(batches: &[RecordBatch]) -> Result<String, ArrowError> {
    let mut bytes = vec![];
    {
        let mut writer = LineDelimitedWriter::new(&mut bytes);
        for batch in batches {
            writer.write(batch)?;
        }
        writer.finish()?;
    }

    let formatted = String::from_utf8(bytes).map_err(|e| ArrowError::JsonError(e.to_string()))?;
    Ok(formatted)
}

fn format_delimited(batches: &[RecordBatch], delimiter: u8) -> Result<String, ArrowError> {
    let mut bytes = vec![];
    {
        let mut writer = CsvWriterBuilder::new()
            .with_header(false)
            .with_delimiter(delimiter)
            .build(&mut bytes);
        for batch in batches {
            writer.write(batch)?;
        }
    }

    let formatted = String::from_utf8(bytes).map_err(|e| ArrowError::CsvError(e.to_string()))?;
    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{Int64Array, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;

    fn sample_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("number", DataType::Int64, false),
            Field::new("label", DataType::Utf8, false),
        ]));

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int64Array::from(vec![1, 2])),
                Arc::new(StringArray::from(vec!["one", "two"])),
            ],
        )
        .unwrap()
    }

    #[test]
    fn formats_table_output() {
        let batches = vec![sample_batch()];
        let result = format_table(&batches);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("| number |"));
    }

    #[test]
    fn formats_json_output() {
        let batches = vec![sample_batch()];
        let result = format_json(&batches);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "{\"number\":1,\"label\":\"one\"}\n{\"number\":2,\"label\":\"two\"}\n"
        );
    }

    #[test]
    fn formats_csv_output() {
        let batches = vec![sample_batch()];
        let result = format_batches(&batches, Output::Csv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1,one\n2,two\n");
    }

    #[test]
    fn formats_tsv_output() {
        let batches = vec![sample_batch()];
        let result = format_batches(&batches, Output::Tsv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1\tone\n2\ttwo\n");
    }

    #[test]
    fn formats_psv_output() {
        let batches = vec![sample_batch()];
        let result = format_batches(&batches, Output::Psv);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1|one\n2|two\n");
    }
}
