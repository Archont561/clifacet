//! `#[clifacet(..)]` on a field: the argument declared once, as grammar plus interaction.
//!
//! An argument in this crate used to be declared four times — an `#[arg(..)]` for the
//! grammar, a field on the input type, a prompt call, and an entry in the form's
//! hand-written field list — and the three declarative ones are what drift. decision-13
//! makes the first the whole declaration: grammar keys are forwarded into clap's
//! `#[arg(..)]`, and the interaction and normalization keys describe what the generated
//! bridge and prompt collection do with the same field.
//!
//! This module owns the grammar half, which is the half that ships first (CLI-28). It
//! forwards tokens rather than reinterpreting them: the value of `short`, `range` or
//! `value_parser` reaches clap exactly as written, so clap stays the grammar authority
//! (decision-2) and this crate still has no second parser.
//!
//! Two rules keep the forwarding honest:
//!
//! - **Known keys only.** An unknown key is a compile error that lists the supported
//!   ones, because a key that is silently ignored is an argument that silently is not
//!   what it says.
//! - **Once each.** A key declared twice is a compile error rather than a last-one-wins
//!   resolution, which is the failure mode a merged attribute would otherwise hide.
//!
//! Anything the key table does not cover stays available as a raw `#[arg(..)]` on the
//! same field, which passes through untouched — the escape hatch decision-13 keeps open.

use syn::punctuated::Punctuated;
use syn::{Attribute, Meta, Token};

/// The attribute this module reads on a field.
pub(crate) const ATTRIBUTE: &str = "clifacet";

/// Grammar keys that take a value, forwarded as `#[arg(key = value)]`.
///
/// Deliberately a list rather than "everything clap knows": it is the documented
/// surface, it is what the error message offers, and a new clap key is added here when
/// someone needs it. The raw `#[arg(..)]` escape hatch covers the rest immediately.
pub(crate) const VALUE_KEYS: &[&str] = &[
    "action",
    "alias",
    "conflicts_with",
    "conflicts_with_all",
    "default_value",
    "default_value_t",
    "env",
    "group",
    "help",
    "id",
    "ignore_case",
    "num_args",
    "range",
    "requires",
    "value_delimiter",
    "value_name",
    "value_parser",
];

/// Grammar keys that are flags, forwarded as `#[arg(key)]`.
pub(crate) const FLAG_KEYS: &[&str] = &[
    "hide",
    "long_alias",
    "required",
    "short_alias",
    "value_enum",
];

/// Grammar keys clap accepts both ways: bare (`long`, deriving the name from the field)
/// or naming the flag explicitly (`long = "greet-name"`). Forwards whichever was written.
pub(crate) const EITHER_KEYS: &[&str] = &["long", "short"];

/// One field's `#[clifacet(..)]`, split into what it means.
pub(crate) struct Declared {
    /// The forwarded `#[arg(..)]` entries, in declaration order.
    arguments: Vec<Meta>,
}

impl Declared {
    /// The `#[arg(..)]` attribute this declaration becomes.
    pub(crate) fn attribute(&self) -> Attribute {
        let entries = &self.arguments;
        syn::parse_quote!(#[arg(#(#entries),*)])
    }
}

/// Parse one `#[clifacet(..)]` attribute, or say what is wrong with it.
pub(crate) fn parse(attribute: &Attribute) -> syn::Result<Declared> {
    let entries = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    if entries.is_empty() {
        return Err(syn::Error::new_spanned(
            attribute,
            format!(
                "`#[{ATTRIBUTE}()]` declares nothing: name at least one key, for example \
                 `#[{ATTRIBUTE}(long, value_name = \"NAME\")]`"
            ),
        ));
    }

    let mut declared: Vec<Meta> = Vec::new();
    for entry in entries {
        let key = entry.path().get_ident().map(ToString::to_string);
        let Some(key) = key else {
            return Err(syn::Error::new_spanned(
                &entry,
                format!("`#[{ATTRIBUTE}(..)]` keys are plain names: {}", supported()),
            ));
        };
        if declared
            .iter()
            .any(|existing| existing.path().is_ident(&key))
        {
            return Err(syn::Error::new(
                entry.path().segments[0].ident.span(),
                format!(
                    "`{key}` is declared twice on this field; one key, one meaning — \
                     merge the two declarations"
                ),
            ));
        }
        declared.push(check_key(entry, &key)?);
    }

    Ok(Declared {
        arguments: declared,
    })
}

/// Accept the entry when the key is one this table forwards, and forward it as written.
fn check_key(entry: Meta, key: &str) -> syn::Result<Meta> {
    let bare = matches!(entry, Meta::Path(_));
    let known = VALUE_KEYS.contains(&key) || FLAG_KEYS.contains(&key) || EITHER_KEYS.contains(&key);
    if !known {
        return Err(unknown(&entry, key));
    }
    if bare && VALUE_KEYS.contains(&key) {
        return Err(syn::Error::new_spanned(
            &entry,
            format!("`{key}` takes a value, for example `{key} = \"…\"`"),
        ));
    }
    if !bare && FLAG_KEYS.contains(&key) {
        return Err(syn::Error::new_spanned(
            &entry,
            format!("`{key}` takes no value: write `{key}` on its own, without `= …`"),
        ));
    }

    // The value is forwarded as tokens: a string, a char, a range or a parser expression
    // all mean what clap says they mean, and this crate does not interpret any of them.
    Ok(entry)
}

fn unknown(entry: &Meta, key: &str) -> syn::Error {
    syn::Error::new_spanned(
        entry,
        format!(
            "`{key}` is not a `#[{ATTRIBUTE}(..)]` key this attribute forwards, so it would \
             be an argument that means nothing. Supported keys: {}. Anything else stays \
             available as a raw `#[arg({key} …)]` on the same field, which this attribute \
             passes through untouched.",
            supported()
        ),
    )
}

fn supported() -> String {
    format!(
        "values — {}; flags — {}; either — {}",
        VALUE_KEYS.join(", "),
        FLAG_KEYS.join(", "),
        EITHER_KEYS.join(", ")
    )
}
