---
name: arrow-cli
description: Use `arrow_cli` to connect to a Flight SQL server and run SQL.
---

# arrow_cli

`arrow_cli` is a command-line client for connecting to a Flight SQL server and executing SQL queries.

## Install

```bash
cargo install arrow_cli
```

After installation, confirm that the binary is available:

```bash
arrow_cli --version
```

## Usage

### Execute one SQL statement and exit

```bash
arrow_cli --host localhost --port 8900 --user admin --password abc --command "select 1"
```

Output:

```text
+----------+
| Int64(1) |
+----------+
| 1        |
+----------+

1 rows in set (tickets received in 0.008 sec, rows received in 0.012 sec)
```

### Pipe SQL through standard input

```bash
echo "select 1" | arrow_cli --host localhost --port 8900 --user admin --password abc
```

Output:

```text
1
```

### Option: print the result schema

```bash
arrow_cli --host localhost --port 8900 --user admin --password abc --print-schema --command "select number from numbers(3);"
```

Output:

```text
+----------+
| Int64(1) |
+----------+
| 1        |
+----------+

Schema {
    fields: [
        Field {
            name: "Int64(1)",
            data_type: Int64,
        },
    ],
    metadata: {},
}

1 rows in set (tickets received in 0.008 sec, rows received in 0.010 sec)
```

## Other notes
- Run `arrow_cli --help` for more details