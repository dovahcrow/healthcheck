# Healthcheck

This lib creates an http server and reports whether your application is healthy
or not through http status code and prometheus metrics.

## Usage

```rust
#[tokio::main]
async fn main() {
    // start an http server on port 9090, report unhealthy if 
    // `HealthCheck::healthy()` is not triggered with 5 secs.
    healthcheck::HealchCheck::init(Duration::from_secs(5), 9090); 

    loop {
        do_some_work().await;
        HealchCheck::healthy().await; // report your application is healthy
    }
}
```

Then have a healthchecker, e.g. `curl --fail http://localhost:9090/healthcheck || exit 1`
to check if the application is healthy.

Alternatively, use prometheus to scrape the metric at `http://localhost:9090/healthcheck/metrics`.
The `healthcheck` metrics will turn `0` if the application is unhealthy.