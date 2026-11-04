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
mod input_type;
mod rules;

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
/// keys as `#[facet(…)]` — `input`, `bridge`, `check` — and exists because a derive cannot add
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
/// - `#[facet(input = Type)]` — required. The typed input the bridge builds: any type
///   the generated impls can name — a qualified path, a generic type, an alias, a
///   projection such as `<T as Trait>::Input`. A lifetime in it must be declared on the
///   command struct, whose generics every generated impl takes, or be `'static`. Without
///   a `bridge` it must name a struct, built as `Self { .. }`. `Self`, `_`, `impl Trait`
///   and `'_` are compile errors at their tokens.
/// - `#[facet(check = path)]` — optional. Called as `path(&input) -> Result<(),
///   Failure>` at the end of the generated `TryFrom`, after every field rule, for a
///   rule across fields.
/// - `#[facet(bridge = path)]` — optional. Called as `path(cli) -> Result<Type,
///   Failure>` instead of the generated `TryFrom`, for input the declarations cannot
///   describe. `check` beside it is a compile error: the bridge runs its own checks.
///
/// On a field, `#[facet(..)]` says how that one value is asked for and normalised:
///
/// - `ask = "…"` / `confirm = "…"` — generated into `clifacet::prompt::Collect`: ask
///   for an `Option` no flag supplied, parsed by the argument's own clap parser; put a
///   set `bool` flag to the user.
/// - `fill = expr`, `trim`, `non_empty`, `secret` — generated into
///   `TryFrom<Self> for Type`, applied in that order. A declared `bridge` owns
///   normalization instead, so these beside it are a compile error.
/// - `skip` — the generated `TryFrom` leaves the field out of the input; clap still
///   parses it. It takes no rule, and a declared `bridge` makes it a compile error.
/// - `form` — the field is listed in `Facet::form_fields`, in declaration order, for a
///   generated TUI form. Like `ask`, it needs a `long` or `short` option.
///
/// Without a `bridge`, the derive implements `TryFrom<Self> for Type` and
/// `validate` delegates to it; fields with no rule are copied by name.
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
