# faf-dns-bench

A (linux-only) tool to benchmark DNS resolution.

Ensure you flush your DNS cache before each use otherwise you'll hit cached answers which invalidate the results.

Sample Output:
![](output.png)

## How to use

`cargo +nightly run --release`
