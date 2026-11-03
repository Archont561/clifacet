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
//!   renamed or missing field a compile error instead of a silent default. Field
//!   rules (`fill`, `trim`, `non_empty`, `secret`) normalise on the way, `skip` leaves
//!   a field out, and `#[facet(check = path)]` runs a rule across the built input;
//! - `#[facet(bridge = path)]`, for an input the declarations cannot describe, which
//!   receives the parsed command and builds the whole input itself.

use quote::{quote, quote_spanned};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::{Data, DeriveInput, Expr, Fields, Ident, Meta, Path, Token};

use crate::rules::{self, Rules};

// The attribute this derive reads, on the struct and on its fields.
use crate::rules::ATTRIBUTE;

/// The keys the attribute accepts.
const SUPPORTED_KEYS: &str = "`input`, `bridge`, `check`";

/// What the `#[facet(...)]` attribute declared.
#[derive(Default)]
struct Declared {
    input: Option<Path>,
    bridge: Option<Path>,
    /// A rule across the built input, run by the generated bridge after the field rules.
    check: Option<Path>,
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
    let name = &input.ident;

    // Either generate the bridge from the field declarations, or hand over to the
    // declared one. A declared bridge receives the parsed command by value and owns
    // its own normalization, which is why a field rule beside it is an error rather
    // than a rule that silently does nothing.
    let (validate, bridge_impl) = if let Some(bridge) = declared.bridge {
        if let Some(check) = &declared.check {
            return Err(syn::Error::new_spanned(
                check,
                "`check` runs inside the generated bridge, and this struct declares its own \
                 `bridge = …`, which builds the whole input: call the check from the bridge, \
                 or drop `bridge` and let the declarations build the input",
            ));
        }
        for field in &fields {
            field.rules.check_bridge()?;
        }
        (quote!(#bridge(self)), quote!())
    } else {
        let assignments = fields
            .iter()
            .filter(|field| field.rules.skip.is_none())
            .map(|field| field.assignment(name));
        // The check sees the input the field rules built, never the raw flags: by the
        // time it runs, every value is filled, trimmed and known not to be blank.
        let check = declared.check.as_ref().map(|check| {
            quote_spanned! {check.span()=>
                let checked: fn(&#input_type) -> ::core::result::Result<(), ::clifacet::Failure> =
                    #check;
                checked(&input)?;
            }
        });
        let bridge_impl = quote! {
            impl ::core::convert::TryFrom<#name> for #input_type {
                type Error = ::clifacet::Failure;

                fn try_from(
                    cli: #name,
                ) -> ::core::result::Result<Self, ::clifacet::Failure> {
                    let input = #input_type { #(#assignments),* };
                    #check
                    ::core::result::Result::Ok(input)
                }
            }
        };
        (
            quote!(<#input_type as ::core::convert::TryFrom<Self>>::try_from(self)),
            bridge_impl,
        )
    };
    let collect = collect_impl(name, &fields);
    let form_fields = form_fields(&fields);

    Ok(quote! {
        impl ::clifacet::Facet for #name {
            type Input = #input_type;

            #form_fields

            fn validate(
                self,
            ) -> ::core::result::Result<#input_type, ::clifacet::Failure> {
                #validate
            }
        }

        #bridge_impl

        #collect
    })
}

/// One field of the command struct, with what it declared.
struct Declaration {
    name: Ident,
    /// The id clap knows the field by, for asking and for naming it in a message.
    id: String,
    rules: Rules,
}

impl Declaration {
    /// The field's entry in the generated struct literal: copied by name, with the
    /// declared rules applied in their one fixed order.
    fn assignment(&self, command: &Ident) -> proc_macro2::TokenStream {
        let name = &self.name;
        if !self.rules.normalises() {
            return quote!(#name: cli.#name);
        }
        let id = &self.id;
        let mut steps = Vec::new();
        if let Some(fallback) = &self.rules.fill {
            steps.push(quote_spanned! {fallback.span()=>
                let value = ::clifacet::facet::__derive::fill(
                    value,
                    || ::core::convert::Into::into(#fallback),
                );
            });
        }
        if let Some(span) = self.rules.trim {
            steps.push(quote_spanned! {span=>
                let value = ::clifacet::facet::__derive::Text::trim(value);
            });
        }
        if let Some(span) = self.rules.non_empty {
            steps.push(quote_spanned! {span=>
                let value = ::clifacet::facet::__derive::Text::non_empty(
                    value,
                    || ::clifacet::facet::__derive::flag::<#command>(#id),
                )?;
            });
        }
        if let Some(span) = self.rules.secret {
            steps.push(quote_spanned! {span=>
                let value = ::clifacet::facet::__derive::credential(value);
            });
        }
        quote!(#name: { let value = cli.#name; #(#steps)* value })
    }
}

/// `Collect` for the command: ask what no flag supplied, confirm what was set.
///
/// Generated for every derived command, with nothing to do when no field declares a
/// question, so collection is one call whatever the command declares.
fn collect_impl(name: &Ident, fields: &[Declaration]) -> proc_macro2::TokenStream {
    let questions: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let (field_name, id) = (&field.name, &field.id);
            if let Some(ask) = &field.rules.ask {
                let text = &ask.text;
                // Spanned at the key: an `ask` on a field that is not an `Option` is a
                // type error, and that is where the user has to look.
                return Some(quote_spanned! {ask.span=>
                    session.ask_for(&mut self.#field_name, &command, #id, #text)?;
                });
            }
            field.rules.confirm.as_ref().map(|confirm| {
                let text = &confirm.text;
                quote_spanned! {confirm.span=>
                    session.confirm_flag(self.#field_name, &command, #id, #text)?;
                }
            })
        })
        .collect();

    if questions.is_empty() {
        return quote! {
            impl ::clifacet::prompt::Collect for #name {
                fn collect(
                    self,
                    _session: &mut ::clifacet::prompt::Session,
                ) -> ::core::result::Result<Self, ::clifacet::Failure> {
                    ::core::result::Result::Ok(self)
                }
            }
        };
    }
    quote! {
        impl ::clifacet::prompt::Collect for #name {
            fn collect(
                mut self,
                session: &mut ::clifacet::prompt::Session,
            ) -> ::core::result::Result<Self, ::clifacet::Failure> {
                let command = <Self as ::clifacet::clap::CommandFactory>::command();
                #(#questions)*
                ::core::result::Result::Ok(self)
            }
        }
    }
}

/// `Facet::form_fields` for the command: every `form` field, in declaration order.
///
/// Nothing is generated without one, so the trait's empty default stays in charge.
fn form_fields(fields: &[Declaration]) -> proc_macro2::TokenStream {
    let ids: Vec<_> = fields
        .iter()
        .filter(|field| field.rules.form.is_some())
        .map(|field| &field.id)
        .collect();
    if ids.is_empty() {
        return quote!();
    }
    quote! {
        fn form_fields() -> &'static [&'static str] {
            &[#(#ids),*]
        }
    }
}

/// Collects the fields the bridge copies, rejecting shapes it cannot bridge.
fn named_fields(input: &DeriveInput) -> syn::Result<Vec<Declaration>> {
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

    named
        .named
        .iter()
        .filter_map(|field| field.ident.clone().map(|name| (field, name)))
        .map(|(field, name)| {
            Ok(Declaration {
                id: rules::argument_id(field, &name),
                rules: rules::parse(field)?,
                name,
            })
        })
        .collect()
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
                "check" => &mut declared.check,
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
                        "`{key}` takes a path, for example \
                         `#[{ATTRIBUTE}({key} = my_module::item)]`"
                    ),
                ));
            };
            *slot = Some(value.path.clone());
        }
    }

    Ok(declared)
}
