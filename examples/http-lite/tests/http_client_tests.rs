//! Integration acceptance tests for `http-lite` against wiremock mock HTTP services (CLI-87).

mod wiremock;

use clifacet::policy::Capabilities;
use clifacet::prompt::Scripted;
use clifacet_http_lite::{HttpLiteApp, Item};
use wiremock::matchers::{method, path, path_prefix};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_app() -> clifacet::App<
    'static,
    HttpLiteApp,
    HttpLiteApp,
    clifacet::app::Plain,
    clifacet_http_lite::HttpContext,
> {
    clifacet::App::for_cli()
}

fn scripted_prompt() -> Box<dyn clifacet::prompt::Prompter> {
    Box::new(Scripted::default())
}

#[derive(Clone, Default)]
struct SharedWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

#[tokio::test]
async fn test_get_command_against_wiremock() {
    let server = MockServer::start().await;

    let item = Item {
        id: "101".into(),
        name: "SuperWidget".into(),
        price: 49,
    };

    Mock::given(method("GET"))
        .and(path("/items/101"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&item))
        .mount(&server)
        .await;

    let app = test_app();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = app
        .run_with_async(
            ["http-lite", "--base-url", &server.uri(), "get", "101"],
            Capabilities::headless(),
            scripted_prompt,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(exit_code, 0);
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("item 101: SuperWidget ($49)"));
}

#[tokio::test]
async fn test_create_command_against_wiremock() {
    let server = MockServer::start().await;

    let created = Item {
        id: "202".into(),
        name: "Gadget".into(),
        price: 99,
    };

    Mock::given(method("POST"))
        .and(path("/items"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&created))
        .mount(&server)
        .await;

    let app = test_app();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = app
        .run_with_async(
            [
                "http-lite",
                "--base-url",
                &server.uri(),
                "create",
                "Gadget",
                "99",
            ],
            Capabilities::headless(),
            scripted_prompt,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(exit_code, 0);
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("created item 202: Gadget ($99)"));
}

#[tokio::test]
async fn test_download_command_with_progress_reporting() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_prefix("/items/303/download"))
        .respond_with(ResponseTemplate::new(200).set_body_string("chunk data"))
        .mount(&server)
        .await;

    let progress_buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let app = test_app().progress_writer(SharedWriter(progress_buf.clone()));
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = app
        .run_with_async(
            [
                "http-lite",
                "--base-url",
                &server.uri(),
                "--json",
                "download",
                "303",
                "--chunks",
                "3",
            ],
            Capabilities::headless(),
            scripted_prompt,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(exit_code, 0);
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("successfully downloaded item 303 in 3 chunks"));

    // Verify progress lines were streamed to the progress sink
    let progress_str = String::from_utf8(progress_buf.lock().unwrap().clone()).unwrap();
    assert!(progress_str.contains("progress"));
    assert!(progress_str.contains("300"));
}

#[tokio::test]
async fn test_slow_fetch_cooperative_cancellation_via_timeout() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/items/500/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("delayed response")
                .insert_header("content-type", "text/plain"),
        )
        .with_delay(std::time::Duration::from_millis(300))
        .mount(&server)
        .await;

    let app = test_app();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = app
        .run_with_async(
            [
                "http-lite",
                "--base-url",
                &server.uri(),
                "--timeout",
                "20ms",
                "slow-fetch",
                "500",
            ],
            Capabilities::headless(),
            scripted_prompt,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(exit_code, 1);
    let err_str = String::from_utf8(err).unwrap();
    assert!(err_str.contains("timed out after 20ms"));
}

#[tokio::test]
async fn test_dry_run_inspection_does_not_execute_mutation() {
    let server = MockServer::start().await;

    // Do NOT register any mock for POST /items; if it makes an HTTP call, the server returns 404
    let app = test_app();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = app
        .run_with_async(
            [
                "http-lite",
                "--base-url",
                &server.uri(),
                "--dry-run",
                "create",
                "GhostItem",
                "10",
            ],
            Capabilities::headless(),
            scripted_prompt,
            &mut out,
            &mut err,
        )
        .await;

    // Commands without explicit preview plans are safely refused before mutation occurs
    assert_eq!(exit_code, 2);
    let err_str = String::from_utf8(err).unwrap();
    assert!(err_str.contains("cannot be run with --dry-run"));
}

#[tokio::test]
async fn test_delete_command_against_wiremock() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/items/777"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let app = test_app();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = app
        .run_with_async(
            ["http-lite", "--base-url", &server.uri(), "delete", "777"],
            Capabilities::headless(),
            scripted_prompt,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(exit_code, 0);
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("deleted item 777"));
}

#[tokio::test]
async fn test_not_found_error_reporting() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/items/999"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let app = test_app();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = app
        .run_with_async(
            ["http-lite", "--base-url", &server.uri(), "get", "999"],
            Capabilities::headless(),
            scripted_prompt,
            &mut out,
            &mut err,
        )
        .await;

    assert_ne!(exit_code, 0);
    let err_str = String::from_utf8(err).unwrap();
    assert!(err_str.contains("item '999' not found"));
}
