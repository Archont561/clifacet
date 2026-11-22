//! Shared application context for `http-lite`.

use crate::client::Client;

/// Configuration and shared client context across all commands.
#[derive(Clone, Debug)]
pub struct HttpContext {
    /// Base URL of the API (e.g. `http://127.0.0.1:8080`).
    pub base_url: String,
    /// HTTP client instance.
    pub client: Client,
    /// Optional bearer token for authentication.
    pub token: Option<String>,
}

impl HttpContext {
    /// Creates a new HTTP context.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            token,
        }
    }
}
