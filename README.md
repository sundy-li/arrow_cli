# arrow_cli &emsp; [![Build Status]][actions] [![Latest Version]][crates.io]

[Build Status]: https://img.shields.io/github/workflow/status/sundy-li/arrow_cli/CI/main
[actions]: https://github.com/sundy-li/arrow_cli/actions?query=branch%3Amain
[Latest Version]: https://img.shields.io/crates/v/arrow_cli.svg
[crates.io]: https://crates.io/crates/arrow_cli



## Overview

arrow_cli is a CLI tool for interacting with server in Flight SQL protocol.

## Install 

```
cargo install arrow_cli
```

## Usage

```
> arrow_cli --help
Usage: arrow_cli <--user <USER>|--password <PASSWORD>|--host <HOST>|--port <PORT>>
```

## Examples

```
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

## Features

- basic highlight
- basic auto-completion
- select query support
- TBD

#### License

<sup>
Licensed under <a href="./LICENSE">Apache License, Version 2.0</a>.
</sup>
