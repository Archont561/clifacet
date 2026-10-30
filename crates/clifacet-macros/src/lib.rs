//! Procedural macros for Clifacet.
//!
//! [`Facet`] is the companion derive decision-2 proposes: it connects a clap
//! command struct to typed input without introducing a second grammar. `version!`
//! reports this crate's package version.
//!
//! `Facet` is re-exported from the `clifacet` library, so applications depend on
//! that crate alone.

mod facet;

use proc_macro::{Literal, TokenStream, TokenTree};

/// Derives `clifacet::Facet`: bridges a clap command struct to typed input.
///
/// # Attributes
///
/// - `#[facet(input = Type)]` — required. The typed input the bridge builds.
/// - `#[facet(validate = path)]` — optional. Called as `path(&input)` after the
///   bridge, for application rules the parser cannot express.
///
/// # Panics
///
/// Panics only if Rust cannot parse fixed, compiler-generated tokens.
#[proc_macro_derive(Facet, attributes(facet))]
pub fn facet(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    facet::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Expands to the version of the `clifacet-macros` package as a string literal.
///
/// # Panics
///
/// Panics if Rust cannot parse a fixed, valid compiler-error expression.
///
/// # Examples
///
/// ```
/// assert!(!clifacet_macros::version!().is_empty());
/// ```
#[proc_macro]
pub fn version(input: TokenStream) -> TokenStream {
    if !input.is_empty() {
        return "compile_error!(\"version! takes no arguments\")"
            .parse()
            .expect("static compile_error tokens are valid");
    }

    TokenTree::Literal(Literal::string(env!("CARGO_PKG_VERSION"))).into()
}

#[cfg(test)]
mod tests;
