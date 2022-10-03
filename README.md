# health-probe
Can be used as a side-car container to provider a /health-probe GET API for health checking.


Inspired by: https://github.com/zq-david-wang/linux-tools/blob/main/misc/tcpportcheck.c

## Usage
```bash
/ # health-probe --help
Usage: health-probe [OPTIONS]

Options:
  -l, --listen-port <LISTEN_PORT>  port to listen on [default: 80]
  -p, --probe-addr <PROBE_ADDR>    address to probe [default: 0.0.0.0:8085]
  -h, --help                       Print help information
  -V, --version                    Print version information
```
