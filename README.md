# arrow_cli &emsp; 

[![Build Status](https://github.com/sundy-li/arrow_cli/actions/workflows/ci.yml/badge.svg)](https://github.com/sundy-li/arrow_cli/actions/workflows/ci.yml)
[![](https://img.shields.io/crates/v/arrow_cli.svg)](https://crates.io/crates/arrow_cli)
[![](https://img.shields.io/crates/d/arrow_cli.svg)](https://crates.io/crates/arrow_cli)


## Overview

arrow_cli is a CLI tool for interacting with server in Flight SQL protocol.

## Install 

```sh
cargo install arrow_cli
```

## Usage

```
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
  -h, --help                 Print help
```

## Examples

### REPL
```sql
❯ arrow_cli -h arch -u sundy -p abc --port 8900
Welcome to Arrow CLI.
Connecting to http://arch:8900/ as user sundy.

arch :) select avg(number) from numbers(10);

select avg(number) from numbers(10);

+-------------+
| avg(number) |
+-------------+
| 4.5         |
+-------------+

1 rows in set (0.036 sec)

arch :) show tables like 'c%';

show tables like 'c%';

+-------------------+
| tables_in_default |
+-------------------+
| customer          |
+-------------------+

1 rows in set (0.030 sec)

arch :) exit
Bye
```

### StdIn Pipe

```bash
❯ echo "select number from numbers(3)" | arrow_cli -h arch -u sundy -p abc --port 8900
0
1
2
```

## Features

- basic keywords highlight
- basic auto-completion
- select query support
- TBD

#### License

<sup>
Licensed under <a href="./LICENSE">Apache License, Version 2.0</a>.
</sup>
