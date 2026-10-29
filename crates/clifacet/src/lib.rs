//! Clifacet's initial library scaffold.
//!
//! Command and presentation APIs will be added in later milestones.

/// The version of the `clifacet` package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of the `clifacet` package.
#[must_use]
pub const fn version() -> &'static str {
    VERSION
}

/// Expands to the version of the `clifacet-macros` package.
pub use clifacet_macros::version;

#[cfg(test)]
mod tests;
