//! `#[derive(Facet)]`: bridge a clap command to typed, validated input.
//!
//! The derive is deliberately thin, because decision-2 keeps `clap` the grammar
//! authority. It implements `clifacet::Facet`, whose `command()` is clap's own
//! [`Command`][clap::Command], and whose `validate` produces the typed input the
//! command accepts. It never builds a second parser, never re-parses argv and
//! never reaches into clap internals: everything a caller observes about flags,
//! aliases, defaults and value parsers is clap's behavior.
//!
//! There are two ways to produce the input, and only two:
//!
//! - the default, a field-by-field mirror of the command struct, which makes a
//!   renamed or missing field a compile error instead of a silent default;
//! - `#[facet(bridge = path)]`, for an input that normalizes — `Option<String>`
//!   to a trimmed `String`, say — and so cannot be built by copying fields.

use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Data, DeriveInput, Expr, Fields, Meta, Path, Token};

/// The attribute this derive reads.
const ATTRIBUTE: &str = "facet";

/// The keys the attribute accepts.
const SUPPORTED_KEYS: &str = "`input`, `bridge`";

/// What the `#[facet(...)]` attribute declared.
#[derive(Default)]
struct Declared {
    input: Option<Path>,
    bridge: Option<Path>,
}

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = named_fields(input)?;
    let declared = parse_attribute(input)?;
    let input_type = declared.input.ok_or_else(|| {
        syn::Error::new_spanned(
            input,
            "the Facet derive needs the typed input to build: \
             add `#[facet(input = YourInputType)]`",
        )
    })?;

    // Either copy the fields across, or hand over to the declared bridge. The
    // bridge receives the parsed command by value and does its own validation,
    // which is what keeps the normalization rules next to the input type.
    let build = if let Some(bridge) = declared.bridge {
        quote!(#bridge(self))
    } else {
        let assignments = fields.iter().map(|field| quote!(#field: self.#field));
        quote!(::core::result::Result::Ok(#input_type { #(#assignments),* }))
    };

    let name = &input.ident;
    Ok(quote! {
        impl ::clifacet::Facet for #name {
            type Input = #input_type;

            fn validate(
                self,
            ) -> ::core::result::Result<#input_type, ::clifacet::Failure> {
                #build
            }
        }
    })
}

/// Collects the field names the bridge copies, rejecting shapes it cannot bridge.
fn named_fields(input: &DeriveInput) -> syn::Result<Vec<syn::Ident>> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "the Facet derive bridges a clap command struct to typed input; \
             enums and unions are not supported",
        ));
    };

    let Fields::Named(named) = &data.fields else {
        return Err(syn::Error::new_spanned(
            input,
            "the Facet derive bridges a command struct field by field; \
             tuple and unit structs are not supported",
        ));
    };

    Ok(named
        .named
        .iter()
        .filter_map(|field| field.ident.clone())
        .collect())
}

/// Reads the `#[facet(...)]` attribute, rejecting anything it cannot honor.
fn parse_attribute(input: &DeriveInput) -> syn::Result<Declared> {
    let mut declared = Declared::default();

    for attribute in &input.attrs {
        if !attribute.path().is_ident(ATTRIBUTE) {
            continue;
        }
        let entries = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for entry in entries {
            let Meta::NameValue(entry) = entry else {
                return Err(syn::Error::new_spanned(
                    &entry,
                    format!(
                        "`#[{ATTRIBUTE}(...)]` takes `key = value` pairs; \
                         supported keys: {SUPPORTED_KEYS}"
                    ),
                ));
            };
            let Some(key) = entry.path.get_ident() else {
                return Err(syn::Error::new_spanned(
                    &entry.path,
                    format!(
                        "unsupported `#[{ATTRIBUTE}(...)]` key; supported keys: {SUPPORTED_KEYS}"
                    ),
                ));
            };
            let slot = match key.to_string().as_str() {
                "input" => &mut declared.input,
                "bridge" => &mut declared.bridge,
                unknown => {
                    return Err(syn::Error::new_spanned(
                        key,
                        format!(
                            "unsupported `#[{ATTRIBUTE}(...)]` key `{unknown}`; \
                             supported keys: {SUPPORTED_KEYS}"
                        ),
                    ));
                }
            };
            if slot.is_some() {
                return Err(syn::Error::new_spanned(
                    key,
                    format!("`{key}` is declared more than once"),
                ));
            }
            let Expr::Path(value) = &entry.value else {
                return Err(syn::Error::new_spanned(
                    &entry.value,
                    format!(
                        "`{key}` takes a path, for example                          `#[{ATTRIBUTE}({key} = my_module::item)]`"
                    ),
                ));
            };
            *slot = Some(value.path.clone());
        }
    }

    Ok(declared)
}
