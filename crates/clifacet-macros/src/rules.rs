//! A field's `#[facet(..)]`: how its value is asked for and normalised.
//!
//! The struct-level `#[facet(input = …)]` says *what* the bridge builds; these
//! field-level keys say what happens to one field on the way. They are the
//! interaction and normalization half of decision-13 — `#[clifacet(..)]` forwards them
//! here, so the derive stays the one place that generates a bridge.
//!
//! | Key | Kind | Generated into |
//! | --- | --- | --- |
//! | `ask = "…"` | interaction | `Collect::collect`: asks when no flag supplied the value |
//! | `confirm = "…"` | interaction | `Collect::collect`: confirms a flag that was set |
//! | `fill = expr` | normalization | the bridge: the value when nothing supplied one |
//! | `trim` | normalization | the bridge: surrounding whitespace removed |
//! | `non_empty` | normalization | the bridge: blank text rejected, or made absent if optional |
//! | `secret` | normalization | the bridge: the field must be a redacted type |
//!
//! The bridge applies its rules in one fixed order — `fill`, `trim`, `non_empty`,
//! `secret` — whatever order they were written in, because "trim, then check blank" is
//! the only order in which `non_empty` means anything.

use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, ExprLit, Field, Lit, LitStr, Meta, Token};

/// The attribute these rules live in, shared with the struct-level keys.
pub(crate) const ATTRIBUTE: &str = "facet";

/// Field keys that take a value.
pub(crate) const VALUE_KEYS: &[&str] = &["ask", "confirm", "fill"];

/// Field keys that are flags.
pub(crate) const FLAG_KEYS: &[&str] = &["non_empty", "secret", "trim"];

/// Keys that need the struct's own bridge, because a declared `bridge` owns them.
const NORMALIZATION_KEYS: &[&str] = &["fill", "non_empty", "secret", "trim"];

/// A question and where it was declared, so a type error can point at the key.
pub(crate) struct Question {
    pub(crate) text: LitStr,
    pub(crate) span: Span,
}

/// Everything one field declared, already checked for shape and combination.
#[derive(Default)]
pub(crate) struct Rules {
    pub(crate) ask: Option<Question>,
    pub(crate) confirm: Option<Question>,
    pub(crate) fill: Option<Expr>,
    pub(crate) trim: Option<Span>,
    pub(crate) non_empty: Option<Span>,
    pub(crate) secret: Option<Span>,
    /// The first normalization key, for the "a bridge owns this" error.
    normalization: Option<(String, Span)>,
}

impl Rules {
    /// Whether the generated bridge has anything to do beyond copying the field.
    pub(crate) fn normalises(&self) -> bool {
        self.normalization.is_some()
    }

    /// Rejects normalization when the struct declared its own bridge.
    pub(crate) fn check_bridge(&self) -> syn::Result<()> {
        match &self.normalization {
            Some((key, span)) => Err(syn::Error::new(
                *span,
                format!(
                    "`{key}` is a rule for the generated bridge, and this struct declares its \
                     own `bridge = …`, which owns normalization: apply the rule in the bridge, \
                     or drop `bridge` and let the declarations build the input"
                ),
            )),
            None => Ok(()),
        }
    }
}

/// Reads every `#[facet(..)]` on a field into its rules.
pub(crate) fn parse(field: &Field) -> syn::Result<Rules> {
    let mut rules = Rules::default();
    for attribute in field.attrs.iter().filter(|a| a.path().is_ident(ATTRIBUTE)) {
        let entries = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for entry in entries {
            read(&mut rules, &entry)?;
        }
    }

    if let (Some(ask), Some(_)) = (&rules.ask, &rules.confirm) {
        return Err(syn::Error::new(
            ask.span,
            "a field is either asked for (`ask`, a value no flag supplied) or confirmed \
             (`confirm`, a flag that was set), not both",
        ));
    }
    if let (Some(ask), Some(_)) = (&rules.ask, &rules.secret) {
        return Err(syn::Error::new(
            ask.span,
            "`ask` on a `secret` would echo the credential as it is typed, and no frontend \
             hides an answer yet: supply it with the flag or its environment variable",
        ));
    }
    if let Some(ask) = &rules.ask
        && !is_named_option(&field.attrs)
    {
        return Err(syn::Error::new(
            ask.span,
            "`ask` needs a named option — `long` or `short` — because the answer is parsed \
             as that flag's value; a positional argument cannot be asked for",
        ));
    }
    Ok(rules)
}

/// Records one key, rejecting unknown keys, repeats and the wrong shape.
fn read(rules: &mut Rules, entry: &Meta) -> syn::Result<()> {
    let Some(ident) = entry.path().get_ident() else {
        return Err(syn::Error::new_spanned(entry, unknown_message("this key")));
    };
    let key = ident.to_string();
    let span = ident.span();
    // Each shape goes to the helper that needs it, so a value key always has its value
    // and a flag key never does — there is no "missing value" case left to handle.
    let repeated = match entry {
        Meta::NameValue(pair) if VALUE_KEYS.contains(&key.as_str()) => {
            record_value(rules, &key, span, &pair.value)?
        }
        Meta::Path(_) if FLAG_KEYS.contains(&key.as_str()) => record_flag(rules, &key, span),
        Meta::Path(_) if VALUE_KEYS.contains(&key.as_str()) => {
            return Err(syn::Error::new(
                span,
                format!("`{key}` takes a value, for example `{key} = \"…\"`"),
            ));
        }
        _ if FLAG_KEYS.contains(&key.as_str()) => {
            return Err(syn::Error::new(
                span,
                format!("`{key}` takes no value: write `{key}` on its own, without `= …`"),
            ));
        }
        _ => return Err(syn::Error::new(span, unknown_message(&format!("`{key}`")))),
    };
    if repeated {
        return Err(syn::Error::new(
            span,
            format!("`{key}` is declared twice on this field; one key, one meaning"),
        ));
    }
    if NORMALIZATION_KEYS.contains(&key.as_str()) && rules.normalization.is_none() {
        rules.normalization = Some((key, span));
    }
    Ok(())
}

/// Records a value key, returning whether it was already declared.
fn record_value(rules: &mut Rules, key: &str, span: Span, value: &Expr) -> syn::Result<bool> {
    Ok(match key {
        "ask" => rules.ask.replace(question(value, span)?).is_some(),
        "confirm" => rules.confirm.replace(question(value, span)?).is_some(),
        _ => rules.fill.replace(value.clone()).is_some(),
    })
}

/// Records a flag key, returning whether it was already declared.
fn record_flag(rules: &mut Rules, key: &str, span: Span) -> bool {
    match key {
        "trim" => rules.trim.replace(span).is_some(),
        "non_empty" => rules.non_empty.replace(span).is_some(),
        _ => rules.secret.replace(span).is_some(),
    }
}

/// A question has to be a string literal: it is shown to a person as written.
fn question(value: &Expr, span: Span) -> syn::Result<Question> {
    match value {
        Expr::Lit(ExprLit {
            lit: Lit::Str(text),
            ..
        }) => Ok(Question {
            text: text.clone(),
            span,
        }),
        other => Err(syn::Error::new_spanned(
            other,
            "a question is a string literal",
        )),
    }
}

fn unknown_message(key: &str) -> String {
    format!(
        "{key} is not a field key of `#[{ATTRIBUTE}(..)]`; supported: {}, {}",
        VALUE_KEYS
            .iter()
            .map(|key| format!("`{key} = …`"))
            .collect::<Vec<_>>()
            .join(", "),
        FLAG_KEYS
            .iter()
            .map(|key| format!("`{key}`"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// Whether the field's `#[arg(..)]` names it `long` or `short`, i.e. it is not positional.
fn is_named_option(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("arg"))
        .filter_map(|attribute| {
            attribute
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .any(|entry| entry.path().is_ident("long") || entry.path().is_ident("short"))
}

/// The id clap gives the field: an explicit `#[arg(id = "…")]`, or the field's name.
pub(crate) fn argument_id(field: &Field, name: &syn::Ident) -> String {
    field
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("arg"))
        .filter_map(|attribute| {
            attribute
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .find_map(|entry| match entry {
            // Only a string literal names the id here; clap rejects anything else itself.
            Meta::NameValue(syn::MetaNameValue {
                path,
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(text),
                        ..
                    }),
                ..
            }) if path.is_ident("id") => Some(text.value()),
            _ => None,
        })
        .unwrap_or_else(|| name.to_string())
}
