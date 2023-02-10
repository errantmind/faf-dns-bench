# faf-dns-bench

A (linux-only) tool to benchmark DNS resolution.

Ensure you flush your DNS cache before each use otherwise you'll hit cached answers which invalidate the results.

Sample Output:

![](output.png)

## How To Use This

`cargo +nightly run --release`

```
Usage: faf-dns-bench [OPTIONS]

Options:
  -d, --debug            debug, default: false
  -s, --server <SERVER>  specify server and port [default: 127.0.0.1:53]
  -h, --help             Print help
  -V, --version          Print version
```
