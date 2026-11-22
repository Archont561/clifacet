# `clifacet-http-lite`

An acceptance example demonstrating an asynchronous HTTP client CLI built on the published `clifacet` architecture (decision-26, milestone `m-21`, CLI-87).

## Features Demonstrated

1. **Async Controllers & Shared Context:**
   - Root application controller using `#[clifacet::app]`.
   - Shared HTTP context initialized via `#[clifacet(context)]` carrying the base URL and HTTP client.
   - Asynchronous subcommands declared in `#[clifacet::commands]`.

2. **Reqwest-Compatible API Against Wiremock:**
   - Communicates asynchronously with HTTP endpoints.
   - Tested against local HTTP mock services matching wiremock expectations.

3. **Cooperative Cancellation & Timeouts:**
   - Long-running network operations inspect `run.cancel().cancelled()` and abort cleanly.
   - Cooperative cancellation yields exit code 130 on cancellation; `--timeout` yields exit code 1 on timeout.

4. **Progress Reporting:**
   - Incremental progress reporting during chunked resource transfers via `run.progress().advance()` and `run.progress().total()`.
   - Streams NDJSON progress events under `--json`.

5. **Dry-Run Inspection:**
   - `--dry-run` inspection verifies the execution plan without issuing mutating HTTP calls.

## Usage

```bash
# Fetch an item by ID
http-lite --base-url http://api.example.com get 101

# Create an item
http-lite --base-url http://api.example.com create "Widget" 49

# Download with progress
http-lite --base-url http://api.example.com download 303 --chunks 4

# Dry run inspection
http-lite --dry-run create "Widget" 49
```
