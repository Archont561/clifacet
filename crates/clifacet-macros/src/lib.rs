//! Procedural macros for Clifacet.
//!
//! [`Facet`] is the companion derive decision-2 proposes: it connects a clap
//! command struct to typed input without introducing a second grammar. [`command`]
//! is the same pairing in one attribute, for callers who would rather not repeat
//! two derives that have to agree. `version!` reports this crate's package version.
//!
//! `Facet` is re-exported from the `clifacet` library, so applications depend on
//! that crate alone.

mod command;
mod facet;
mod field;

use proc_macro::{Literal, TokenStream, TokenTree};

/// Wraps a struct in clap's parser and the [`Facet`] bridge, in one attribute.
///
/// ```ignore
/// #[clifacet::command(input = Greeting)]
/// #[command(name = "greet", about = "Say hello")]
/// struct Cli { #[arg(long)] name: Option<String> }
/// ```
///
/// Expands to the item exactly as written, preceded by
/// `#[derive(::clap::Parser, ::clifacet::Facet)]` and `#[facet(…)]`. It takes the same
/// keys as `#[facet(…)]` — `input`, `bridge` — and exists because a derive cannot add
/// another derive to the same item.
///
/// `clap` still has to be a direct dependency of the calling crate: `clap`'s derive
/// expands to code naming `clap::…`, which an attribute cannot redirect. `clifacet`
/// re-exports `clap`, so the version is the one this macro was built against.
///
/// # Panics
///
/// Panics only if Rust cannot parse fixed, compiler-generated tokens.
#[proc_macro_attribute]
pub fn command(attribute: TokenStream, item: TokenStream) -> TokenStream {
    command::expand(&attribute.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives `clifacet::Facet`: bridges a clap command struct to typed input.
///
/// # Attributes
///
/// - `#[facet(input = Type)]` — required. The typed input the bridge builds.
/// - `#[facet(bridge = path)]` — optional. Called as `path(cli) -> Result<Type,
///   Failure>` instead of copying fields, for input that normalizes on the way in.
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
