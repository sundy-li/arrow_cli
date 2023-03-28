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
Usage: arrow_cli <--user <USER>|--password <PASSWORD>|--host <HOST>|--port <PORT>>
```

## Examples

### REPL
```sql
❯ arrow_cli -h arch -u sundy -p abc --port 8900
arrow_cli :) select avg(number) from numbers(10);

select avg(number) from numbers(10);

+-------------+
| avg(number) |
+-------------+
| 4.5         |
+-------------+
arrow_cli :) show tables like 'c%';

show tables like 'c%';

+-------------------+
| tables_in_default |
+-------------------+
| customer          |
+-------------------+
arrow_cli :) 
```

### StdIn Pipe

```bash
❯ echo "select number from numbers(3)" | arrow_cli -h arch -u sundy -p abc --port 8900
0
1
2
```

## Features

- basic highlight
- basic auto-completion
- select query support
- TBD

#### License

<sup>
Licensed under <a href="./LICENSE">Apache License, Version 2.0</a>.
</sup>
