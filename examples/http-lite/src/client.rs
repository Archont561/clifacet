//! Asynchronous HTTP client implementation matching the `reqwest` surface.

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// HTTP status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(pub u16);

impl StatusCode {
    /// 200 OK.
    pub const OK: Self = Self(200);
    /// 201 Created.
    pub const CREATED: Self = Self(201);
    /// 204 No Content.
    pub const NO_CONTENT: Self = Self(204);
    /// 400 Bad Request.
    pub const BAD_REQUEST: Self = Self(400);
    /// 404 Not Found.
    pub const NOT_FOUND: Self = Self(404);

    /// Returns whether this status is success (200..=299).
    #[must_use]
    pub fn is_success(self) -> bool {
        (200..=299).contains(&self.0)
    }

    /// Returns the numerical code.
    #[must_use]
    pub fn as_u16(self) -> u16 {
        self.0
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// HTTP client error.
#[derive(Debug)]
pub enum Error {
    /// I/O error occurred during network communication.
    Io(io::Error),
    /// Error serializing or deserializing JSON.
    Json(serde_json::Error),
    /// Invalid URL syntax.
    Url(String),
    /// Response parsing error.
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "network I/O error: {e}"),
            Self::Json(e) => write!(f, "JSON error: {e}"),
            Self::Url(s) => write!(f, "invalid URL: {s}"),
            Self::Parse(s) => write!(f, "response parsing error: {s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Url(_) | Self::Parse(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// Asynchronous HTTP client.
#[derive(Clone, Debug, Default)]
pub struct Client;

impl Client {
    /// Creates a new HTTP client.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Builds a `GET` request.
    #[must_use]
    pub fn get(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), "GET", url)
    }

    /// Builds a `POST` request.
    #[must_use]
    pub fn post(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), "POST", url)
    }

    /// Builds a `DELETE` request.
    #[must_use]
    pub fn delete(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), "DELETE", url)
    }
}

/// Builder for constructing and sending HTTP requests.
pub struct RequestBuilder {
    _client: Client,
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    fn new(client: Client, method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            _client: client,
            method: method.into(),
            url: url.into(),
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Adds a request header.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Sets a JSON body on the request.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Json`] if serialization fails.
    pub fn json<T: serde::Serialize>(mut self, data: &T) -> Result<Self, Error> {
        let bytes = serde_json::to_vec(data)?;
        self.headers
            .insert("content-type".into(), "application/json".into());
        self.body = Some(bytes);
        Ok(self)
    }

    /// Sets a raw byte body.
    #[must_use]
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sends the request asynchronously and returns the response.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] on connection or protocol failure.
    pub async fn send(self) -> Result<Response, Error> {
        tokio::task::spawn_blocking(move || self.send_blocking())
            .await
            .map_err(|e| Error::Io(io::Error::other(e)))?
    }

    fn send_blocking(self) -> Result<Response, Error> {
        use std::fmt::Write as _;

        let parsed = parse_url(&self.url)?;
        let host_port = format!("{}:{}", parsed.host, parsed.port);
        let mut stream = TcpStream::connect(&host_port)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        let body = self.body.unwrap_or_default();
        let mut req_header = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: clifacet-http-lite/0.9.0\r\nConnection: close\r\nContent-Length: {}\r\n",
            self.method,
            parsed.path_and_query,
            host_port,
            body.len()
        );
        for (k, v) in &self.headers {
            let _ = write!(req_header, "{k}: {v}\r\n");
        }
        req_header.push_str("\r\n");

        stream.write_all(req_header.as_bytes())?;
        if !body.is_empty() {
            stream.write_all(&body)?;
        }
        stream.flush()?;

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf)?;

        parse_response(&buf)
    }
}

/// HTTP response received from a server.
#[derive(Debug, Clone)]
pub struct Response {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Response {
    /// Returns the HTTP status code.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns response headers.
    #[must_use]
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Returns the response body decoded as a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parse`] if the body is not valid UTF-8.
    #[allow(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "matches reqwest::Response::text async API"
    )]
    pub async fn text(self) -> Result<String, Error> {
        String::from_utf8(self.body).map_err(|e| Error::Parse(e.to_string()))
    }

    /// Deserializes the response body as JSON.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Json`] if deserialization fails.
    #[allow(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "matches reqwest::Response::json async API"
    )]
    pub async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T, Error> {
        serde_json::from_slice(&self.body).map_err(Error::from)
    }

    /// Returns the raw response bytes.
    #[must_use]
    pub fn bytes(self) -> Vec<u8> {
        self.body
    }
}

struct ParsedUrl {
    host: String,
    port: u16,
    path_and_query: String,
}

fn parse_url(url: &str) -> Result<ParsedUrl, Error> {
    let stripped = url
        .strip_prefix("http://")
        .ok_or_else(|| Error::Url("only http:// URLs supported in tests".into()))?;

    let (host_port, path_part) = match stripped.find('/') {
        Some(idx) => (&stripped[..idx], &stripped[idx..]),
        None => (stripped, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(idx) => {
            let h = &host_port[..idx];
            let p: u16 = host_port[idx + 1..]
                .parse()
                .map_err(|e| Error::Url(format!("invalid port: {e}")))?;
            (h.to_string(), p)
        }
        None => (host_port.to_string(), 80),
    };

    Ok(ParsedUrl {
        host,
        port,
        path_and_query: path_part.to_string(),
    })
}

fn parse_response(raw: &[u8]) -> Result<Response, Error> {
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| Error::Parse("incomplete HTTP response header".into()))?;

    let header_str = std::str::from_utf8(&raw[..header_end])
        .map_err(|e| Error::Parse(format!("non-utf8 headers: {e}")))?;

    let mut lines = header_str.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| Error::Parse("empty response".into()))?;

    let mut parts = status_line.split_whitespace();
    let _version = parts
        .next()
        .ok_or_else(|| Error::Parse("missing version".into()))?;
    let status_code_str = parts
        .next()
        .ok_or_else(|| Error::Parse("missing status code".into()))?;
    let code: u16 = status_code_str
        .parse()
        .map_err(|e| Error::Parse(format!("invalid status code: {e}")))?;

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    let body = raw[header_end + 4..].to_vec();

    Ok(Response {
        status: StatusCode(code),
        headers,
        body,
    })
}
