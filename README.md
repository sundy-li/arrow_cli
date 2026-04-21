# arrow_cli &emsp; 

[![Build Status](https://github.com/sundy-li/arrow_cli/actions/workflows/ci.yml/badge.svg)](https://github.com/sundy-li/arrow_cli/actions/workflows/ci.yml)
[![](https://img.shields.io/crates/v/arrow_cli.svg)](https://crates.io/crates/arrow_cli)
[![](https://img.shields.io/crates/d/arrow_cli.svg)](https://crates.io/crates/arrow_cli)


## Overview

arrow_cli is a CLI tool for interacting with a server that speaks the Flight SQL protocol.

## Install 

```sh
cargo install arrow_cli
```

## Usage

```text
> arrow_cli --help
Usage: arrow_cli [OPTIONS]

Options:
  -u, --user <USER>          User name [default: root]
  -p, --password <PASSWORD>  User password [default: ]
      --host <HOST>          Flight SQL Server host [default: 127.0.0.1]
  -P, --port <PORT>          Flight SQL Server port [default: 4100]
      --tls
      --timeout <TIMEOUT>    Request timeout in seconds [default: 180]
      --prepared             Execute query using prepared statement
      --print-schema         Print resultset schema
      --output <OUTPUT>      Result output format [default: table] [possible values: table, json, csv, tsv, psv]
  -c, --command <COMMAND>    Execute SQL command and exit
  -h, --help                 Print help
```

## Examples

### Single command with table output
```bash
❯ arrow_cli -h arch -u sundy -p abc --port 8900 --output table --command "select avg(number) from numbers(10);"
+-------------+
| avg(number) |
+-------------+
| 4.5         |
+-------------+

1 rows in set (tickets received in 0.036 sec, rows received in 0.036 sec)
```

### Single command with JSON output

```bash
❯ arrow_cli -h arch -u sundy -p abc --port 8900 --output json --command "select number from numbers(3)"
{"number":0}
{"number":1}
{"number":2}
```

### StdIn pipe with CSV output

```bash
❯ echo "select number from numbers(3)" | arrow_cli -h arch -u sundy -p abc --port 8900 --output csv
0
1
2
```

### StdIn pipe with TSV output

```bash
❯ echo "select number, concat('v', to_string(number)) from numbers(3)" | arrow_cli -h arch -u sundy -p abc --port 8900 --output tsv
0	v0
1	v1
2	v2
```

### Single command with PSV output

```bash
❯ arrow_cli -h arch -u sundy -p abc --port 8900 --output psv --command "select number, concat('v', to_string(number)) from numbers(3)"
0|v0
1|v1
2|v2
```

### Interactive session with JSON output

```text
❯ arrow_cli -h arch -u sundy -p abc --port 8900 --output json
Welcome to Arrow CLI v0.4.1.
Connecting to http://arch:8900/ as user sundy.

arch :) select number from numbers(2);

select number from numbers(2);

{"number":0}
{"number":1}

arch :) exit
Bye
```

## Features

- basic keywords highlight
- basic auto-completion
- select query support
- output formats: table, json, csv, tsv, psv
- delimited formats use: csv=`,`, tsv=`\t`, psv=`|`
- TBD

#### License

<sup>
Licensed under <a href="./LICENSE">Apache License, Version 2.0</a>.
</sup>
