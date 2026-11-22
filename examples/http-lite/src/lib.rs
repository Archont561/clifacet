//! `http-lite`: an asynchronous HTTP client acceptance example built on clifacet (decision-26, CLI-87).
//!
//! Demonstrates:
//! - Native asynchronous command controllers with `#[clifacet::app]` and `#[clifacet::commands]`.
//! - Asynchronous HTTP communication against mock HTTP endpoints.
//! - Cooperative cancellation (`Cancel`) on slow/long-running network calls.
//! - Progress reporting (`ProgressSink`) during multi-part downloads.
//! - `--dry-run` inspection and planning without mutating remote resources.
//! - Shared context carrying the base URL and HTTP client.

pub mod client;
pub mod context;

pub use client::{Client, Error, RequestBuilder, Response, StatusCode};
pub use context::HttpContext;

/// Reqwest-compatible API re-exported for convenience.
pub mod reqwest {
    pub use crate::client::{Client, Error, RequestBuilder, Response, StatusCode};
}

use clifacet::r#async::RunCx;
use clifacet::{Failure, app, commands};
use serde::{Deserialize, Serialize};

/// Representation of an API resource.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Item {
    /// Item identifier.
    pub id: String,
    /// Name of the item.
    pub name: String,
    /// Price in whole units.
    pub price: u64,
}

/// Payload sent when creating a new resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateItemRequest {
    /// Name of the item.
    pub name: String,
    /// Price in whole units.
    pub price: u64,
}

#[allow(clippy::unnecessary_wraps)]
fn build_context(cli: &HttpLiteApp) -> Result<HttpContext, Failure> {
    Ok(HttpContext::new(cli.base_url.clone(), cli.token.clone()))
}

/// The `http-lite` CLI application controller.
#[app(
    name = "http-lite",
    version = "0.9.0",
    about = "Async HTTP client CLI demonstration",
    commands
)]
pub struct HttpLiteApp {
    /// Global flags (--json, --dry-run, --timeout, etc.).
    #[command(flatten)]
    pub globals: clifacet::Globals,

    /// Base URL of the remote API server.
    #[clifacet(long, default_value = "http://127.0.0.1:8080")]
    pub base_url: String,

    /// Optional authentication token.
    #[clifacet(long)]
    pub token: Option<String>,

    /// Shared HTTP context.
    #[clifacet(context = build_context)]
    pub cx: HttpContext,
}

#[commands]
impl HttpLiteApp {
    /// Fetches an item by ID from the remote API.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if the network request fails, or the remote server responds with an error.
    pub async fn get(&self, id: String, _run: &RunCx) -> Result<String, Failure> {
        let url = format!("{}/items/{id}", self.cx.base_url);
        let mut req = self.cx.client.get(&url);
        if let Some(token) = &self.cx.token {
            req = req.header("authorization", format!("Bearer {token}"));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| Failure::execution(format!("HTTP request failed: {e}")))?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Err(Failure::execution(format!("item '{id}' not found")));
        }

        if !resp.status().is_success() {
            return Err(Failure::execution(format!(
                "server responded with status {}",
                resp.status()
            )));
        }

        let item: Item = resp
            .json()
            .await
            .map_err(|e| Failure::execution(format!("failed to parse JSON response: {e}")))?;

        Ok(format!("item {}: {} (${})", item.id, item.name, item.price))
    }

    /// Creates a new item on the remote API.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if the request serialization fails or the server returns an error.
    pub async fn create(&self, name: String, price: u64, _run: &RunCx) -> Result<String, Failure> {
        if self.globals.dry_run {
            return Ok(format!(
                "dry-run: would create item '{name}' with price ${price}"
            ));
        }

        let url = format!("{}/items", self.cx.base_url);
        let payload = CreateItemRequest {
            name: name.clone(),
            price,
        };

        let mut req = self
            .cx
            .client
            .post(&url)
            .json(&payload)
            .map_err(|e| Failure::validation(format!("cannot serialize payload: {e}")))?;

        if let Some(token) = &self.cx.token {
            req = req.header("authorization", format!("Bearer {token}"));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| Failure::execution(format!("HTTP request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(Failure::execution(format!(
                "server responded with status {}",
                resp.status()
            )));
        }

        let created: Item = resp
            .json()
            .await
            .map_err(|e| Failure::execution(format!("failed to parse JSON response: {e}")))?;

        Ok(format!(
            "created item {}: {} (${})",
            created.id, created.name, created.price
        ))
    }

    /// Downloads a resource, reporting incremental progress.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if the download is cancelled or an HTTP request fails.
    pub async fn download(
        &self,
        id: String,
        #[clifacet(long, default_value_t = 4)] chunks: u32,
        run: &RunCx,
    ) -> Result<String, Failure> {
        let total_chunks = u64::from(chunks.max(1));
        run.progress().total(Some(total_chunks * 100));

        let url = format!("{}/items/{id}/download", self.cx.base_url);
        for chunk in 1..=total_chunks {
            if run.cancel().is_cancelled() {
                return Err(Failure::cancelled("download cancelled"));
            }

            let chunk_url = format!("{url}?chunk={chunk}");
            let resp =
                self.cx.client.get(&chunk_url).send().await.map_err(|e| {
                    Failure::execution(format!("failed to fetch chunk {chunk}: {e}"))
                })?;

            if !resp.status().is_success() {
                return Err(Failure::execution(format!(
                    "server returned {} on chunk {chunk}",
                    resp.status()
                )));
            }

            run.progress().advance(100);
            run.progress()
                .message(&format!("downloaded chunk {chunk}/{total_chunks}"));
        }

        Ok(format!(
            "successfully downloaded item {id} in {total_chunks} chunks"
        ))
    }

    /// Fetches a slow-responding resource with cooperative cancellation support.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if cancelled or if the request times out or fails.
    pub async fn slow_fetch(&self, id: String, run: &RunCx) -> Result<String, Failure> {
        let url = format!("{}/items/{id}/slow", self.cx.base_url);
        let req_fut = self.cx.client.get(&url).send();

        tokio::select! {
            () = run.cancel().cancelled() => {
                Err(Failure::cancelled("slow fetch operation was cancelled"))
            }
            res = req_fut => {
                let resp = res.map_err(|e| Failure::execution(format!("HTTP request failed: {e}")))?;
                if !resp.status().is_success() {
                    return Err(Failure::execution(format!("server returned {}", resp.status())));
                }
                let body = resp.text().await.map_err(|e| Failure::execution(e.to_string()))?;
                Ok(format!("slow response: {body}"))
            }
        }
    }

    /// Deletes an item from the remote API.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] if the request fails or the remote server returns an error.
    pub async fn delete(&self, id: String, _run: &RunCx) -> Result<String, Failure> {
        if self.globals.dry_run {
            return Ok(format!("dry-run: would delete item {id}"));
        }

        let url = format!("{}/items/{id}", self.cx.base_url);
        let mut req = self.cx.client.delete(&url);
        if let Some(token) = &self.cx.token {
            req = req.header("authorization", format!("Bearer {token}"));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| Failure::execution(format!("HTTP request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(Failure::execution(format!(
                "server responded with status {}",
                resp.status()
            )));
        }

        Ok(format!("deleted item {id}"))
    }
}
