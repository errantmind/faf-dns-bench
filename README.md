# faf-dns-bench

A (linux-only) tool to benchmark DNS resolution.

Ensure you flush your DNS cache before each use otherwise you'll hit cached answers which invalidate the results.

Sample Output:

![](output.png)

## How To Use This

`cargo +nightly run --release`

```
FaF DNS Bench - A DNS Resolution Benchmarker

Usage: faf-dns-bench [OPTIONS]

Options:
  -d, --debug            enable debug output [default: false]
  -s, --server <SERVER>  e.g. 1.1.1.1 [default: system default is parsed using `nslookup .`]
  -p, --port <PORT>      [default: 53]
  -h, --help             Print help
  -V, --version          Print version
```
