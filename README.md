# faf-dns-bench

A (linux-only) tool to benchmark DNS resolution.

Ensure you flush your DNS cache before each use otherwise you'll hit the cache which will invalidate the results.

## How to use

`cargo +nightly run --release`
