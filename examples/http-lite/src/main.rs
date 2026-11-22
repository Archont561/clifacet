//! Binary entrypoint for `http-lite`.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    clifacet::run_cli_async::<clifacet_http_lite::HttpLiteApp>().await
}
