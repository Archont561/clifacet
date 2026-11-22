//! Lightweight local HTTP mock server matching the `wiremock` API.

#![allow(
    clippy::pedantic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::unused_async,
    clippy::needless_pass_by_value
)]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Local mock HTTP server.
pub struct MockServer {
    addr: SocketAddr,
    mocks: Arc<Mutex<Vec<Mock>>>,
    shutdown: Arc<AtomicBool>,
}

impl MockServer {
    /// Starts a new mock server on an ephemeral local port.
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let addr = listener.local_addr().expect("local addr");
        let mocks: Arc<Mutex<Vec<Mock>>> = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let server_mocks = Arc::clone(&mocks);
        let server_shutdown = Arc::clone(&shutdown);

        thread::spawn(move || {
            while !server_shutdown.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        let mocks = Arc::clone(&server_mocks);
                        thread::spawn(move || {
                            socket.set_read_timeout(Some(Duration::from_secs(5))).ok();
                            socket.set_write_timeout(Some(Duration::from_secs(5))).ok();

                            let mut buf = [0u8; 4096];
                            let n = match socket.read(&mut buf) {
                                Ok(n) if n > 0 => n,
                                _ => return,
                            };
                            let req_str = String::from_utf8_lossy(&buf[..n]);
                            let (method, path) = parse_req_line(&req_str);

                            let mock_match = {
                                let list = mocks.lock().unwrap();
                                list.iter().find(|m| m.matches(&method, &path)).cloned()
                            };

                            let (resp, delay) = if let Some(m) = mock_match {
                                (m.response, m.delay)
                            } else {
                                (
                                    ResponseTemplate::new(404)
                                        .set_body_string("wiremock: no mock matched"),
                                    None,
                                )
                            };

                            if let Some(d) = delay {
                                thread::sleep(d);
                            }

                            let status_text = match resp.status {
                                200 => "OK",
                                201 => "Created",
                                204 => "No Content",
                                400 => "Bad Request",
                                404 => "Not Found",
                                _ => "Custom",
                            };

                            let mut header = format!(
                                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
                                resp.status,
                                status_text,
                                resp.body.len()
                            );
                            for (k, v) in &resp.headers {
                                header.push_str(&format!("{k}: {v}\r\n"));
                            }
                            header.push_str("\r\n");

                            let _ = socket.write_all(header.as_bytes());
                            let _ = socket.write_all(&resp.body);
                            let _ = socket.flush();
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            addr,
            mocks,
            shutdown,
        }
    }

    /// Returns the root base URL (e.g. `http://127.0.0.1:45321`).
    #[must_use]
    pub fn uri(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Registers a mock definition.
    pub async fn register(&self, mock: Mock) {
        self.mocks.lock().unwrap().push(mock);
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

fn parse_req_line(raw: &str) -> (String, String) {
    if let Some(first_line) = raw.lines().next() {
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("GET").to_uppercase();
        let path = parts.next().unwrap_or("/").to_string();
        (method, path)
    } else {
        ("GET".to_string(), "/".to_string())
    }
}

/// A configured mock expectation and response.
#[derive(Clone)]
pub struct Mock {
    method: Option<String>,
    path: Option<String>,
    path_prefix: Option<String>,
    response: ResponseTemplate,
    delay: Option<Duration>,
}

impl Mock {
    /// Begins building a mock with a given matcher.
    #[must_use]
    pub fn given(matcher: impl Matcher) -> Self {
        let mut mock = Self {
            method: None,
            path: None,
            path_prefix: None,
            response: ResponseTemplate::new(200),
            delay: None,
        };
        matcher.apply(&mut mock);
        mock
    }

    /// Adds another condition to match.
    #[must_use]
    pub fn and(mut self, matcher: impl Matcher) -> Self {
        matcher.apply(&mut self);
        self
    }

    /// Configures the response template to return on match.
    #[must_use]
    pub fn respond_with(mut self, response: ResponseTemplate) -> Self {
        self.response = response;
        self
    }

    /// Adds artificial latency before responding.
    #[must_use]
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Mounts the mock onto the specified [`MockServer`].
    pub async fn mount(self, server: &MockServer) {
        server.register(self).await;
    }

    fn matches(&self, method: &str, path: &str) -> bool {
        if let Some(expected_method) = &self.method
            && !expected_method.eq_ignore_ascii_case(method)
        {
            return false;
        }
        if let Some(expected_path) = &self.path
            && expected_path != path
        {
            return false;
        }
        if let Some(expected_prefix) = &self.path_prefix
            && !path.starts_with(expected_prefix)
        {
            return false;
        }
        true
    }
}

/// Trait for matchers configuring a [`Mock`].
pub trait Matcher {
    /// Applies this matcher to the target mock.
    fn apply(&self, mock: &mut Mock);
}

/// Request matchers.
pub mod matchers {
    use super::{Matcher, Mock};

    /// Matches the HTTP method.
    #[must_use]
    pub fn method(m: &str) -> MethodMatcher {
        MethodMatcher(m.to_uppercase())
    }

    /// Matches the request path exactly.
    #[must_use]
    pub fn path(p: &str) -> PathMatcher {
        PathMatcher(p.to_string())
    }

    /// Matches request path by prefix.
    #[must_use]
    pub fn path_prefix(p: &str) -> PathPrefixMatcher {
        PathPrefixMatcher(p.to_string())
    }

    /// Matcher for HTTP methods.
    pub struct MethodMatcher(String);
    impl Matcher for MethodMatcher {
        fn apply(&self, mock: &mut Mock) {
            mock.method = Some(self.0.clone());
        }
    }

    /// Matcher for exact HTTP paths.
    pub struct PathMatcher(String);
    impl Matcher for PathMatcher {
        fn apply(&self, mock: &mut Mock) {
            mock.path = Some(self.0.clone());
        }
    }

    /// Matcher for HTTP path prefixes.
    pub struct PathPrefixMatcher(String);
    impl Matcher for PathPrefixMatcher {
        fn apply(&self, mock: &mut Mock) {
            mock.path_prefix = Some(self.0.clone());
        }
    }
}

/// A configured response template returned by a mock.
#[derive(Clone, Debug)]
pub struct ResponseTemplate {
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl ResponseTemplate {
    /// Creates a response template with status code.
    #[must_use]
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Sets a JSON body and `content-type: application/json`.
    #[must_use]
    pub fn set_body_json<T: serde::Serialize>(mut self, val: &T) -> Self {
        self.headers
            .insert("content-type".into(), "application/json".into());
        self.body = serde_json::to_vec(val).unwrap_or_default();
        self
    }

    /// Sets a raw string body.
    #[must_use]
    pub fn set_body_string(mut self, s: &str) -> Self {
        self.body = s.as_bytes().to_vec();
        self
    }

    /// Inserts a custom header.
    #[must_use]
    pub fn insert_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }
}
