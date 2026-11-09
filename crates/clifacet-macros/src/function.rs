//! `#[clifacet::command]` on a function: the same command, with the parameters as the declaration.
//!
//! The struct form declares a command twice by necessity — a `clap` struct for the grammar and
//! a separate input type for what the command runs on — and then bridges them. A function
//! carries a body, so both can be derived from its parameters, and the only thing an author
//! writes is the signature, the help text and the work.
//!
//! ```ignore
//! /// Rename a task.
//! #[clifacet::command]
//! pub fn rename(
//!     #[clifacet(long, help = "The task to rename")]
//!     id: u32,
//!     #[clifacet(long, help = "The new title")]
//!     title: String,
//! ) -> Result<String, ::clifacet::Failure> { .. }
//! ```
//!
//! The function is emitted untouched. Beside it come the type `commands!` looks for — because
//! `list.rs` registers as `list::List` — and the input type, both built from the parameters
//! field for field, so `#[clifacet(..)]` means here exactly what it means on a struct field.
//! The function keeps its own name: clap rejects a variant named `List` holding a struct called
//! `List`, and `list(...)` is a call, not a type.
//!
//! When the command needs the application's context, the context is the last parameter:
//!
//! ```ignore
//! #[clifacet::command]
//! pub fn add(title: String, cx: &App) -> Result<usize, ::clifacet::Failure> { .. }
//! ```
//!
//! A reference is the context **only** when the folder's `commands!(..)` declares
//! `context = App`. Nothing is inferred from the type: an `&App` with no declared context has
//! no value to borrow, and answering that with a borrow error three levels down would hide a
//! mistake this crate can name in one line. The parameter must be last, so that every
//! parameter before it is input.
//!
//! # What a parameter means
//!
//! - a bare parameter is a **positional**, exactly as a bare struct field is;
//! - `#[clifacet(long, ..)]` makes it a flag, and a flag without `help` is refused, because a
//!   parameter cannot carry a doc comment — `help` is the only place its description can live;
//! - `#[clifacet(..)]` and `#[arg(..)]` are forwarded exactly as the field form forwards them.
//!
//! The function's own doc comment moves to the generated struct, where clap reads it the way it
//! reads a struct's: the first paragraph is `about`, the whole comment is `long_about`.

use heck::ToUpperCamelCase as _;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::parse::Parser as _;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, FnArg, GenericArgument, ItemFn, Meta, Pat, PathArguments, ReturnType, Token, Type,
};

use crate::field;

/// The derives the generated struct carries.
///
/// `Debug` and `Clone` are what `commands!` asks of a leaf, `Parser` is clap's grammar, and
/// `Facet` is what writes `validate` — from the same field declarations the struct form uses,
/// so this attribute cannot drift from the derive.
const DERIVES: &str = "::core::fmt::Debug, ::core::clone::Clone, ::clap::Parser, ::clifacet::Facet";

/// The same as [`DERIVES`], as an attribute, because a derive takes paths and not a string.
fn derives() -> TokenStream {
    let derives = DERIVES.split(", ").map(|path| {
        let path: syn::Path = syn::parse_str(path).expect("every derive here is a path");
        quote!(#path)
    });
    quote!(#[derive(#(#derives),*)])
}

/// Expand `#[clifacet::command]` on a function, or say what to write instead.
pub(crate) fn expand(attribute: &TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut parsed = syn::parse2::<ItemFn>(item)?;
    // The signature is read first, then rewritten: a parameter's attributes are the
    // declaration, and Rust does not accept them on a `fn` parameter, so they move into the
    // generated struct rather than staying where they were written.
    let (name, output) = {
        let signature = &parsed.sig;
        // The attribute's own keys come first: a struct-only key such as `bridge` would
        // otherwise be accepted here and mean nothing.
        check_attribute(attribute)?;
        check_signature(signature)?;

        let ReturnType::Type(_, returned) = &signature.output else {
            // `check_signature` has already rejected this, with a message about the return.
            return Err(syn::Error::new_spanned(signature, "unreachable"));
        };
        let (output, _) = result_of(returned).ok_or_else(|| {
            syn::Error::new_spanned(
                returned,
                "a function command returns `Result<Output, ::clifacet::Failure>`",
            )
        })?;
        (signature.ident.clone(), output)
    };
    let leaf = pascal(&name.to_string());
    let input = syn::Ident::new(&format!("{leaf}Input"), Span::call_site());
    let documentation: Vec<Attribute> = parsed
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("doc"))
        .cloned()
        .collect();
    let (arguments, context) = parameters(&mut parsed.sig)?;
    let derives = derives();

    let struct_fields = arguments.iter().map(Argument::field);
    let input_fields = arguments.iter().map(Argument::input_field);
    let patterns = arguments
        .iter()
        .map(|argument| &argument.pattern)
        .collect::<Vec<_>>();
    let call = match &context {
        Some(_) if patterns.is_empty() => quote!(#name(cx)),
        Some(_) => quote!(#name(#(#patterns),*, cx)),
        None => quote!(#name(#(#patterns),*)),
    };
    let helper_parameters = arguments
        .iter()
        .map(|argument| {
            let (pattern, ty) = (&argument.pattern, &argument.ty);
            quote!(#pattern: #ty)
        })
        .collect::<Vec<_>>();
    // The context is appended to the same list, so an empty command and a context-only one
    // build the same way: one list of parameters, whatever it holds.
    let mut helper_parameters = helper_parameters;
    if let Some(context) = &context {
        helper_parameters.push(quote!(cx: &#context));
    }

    let implementation = implementation(&leaf, &input, &output, context.as_ref(), &patterns);

    // The `Result` is this macro's requirement, not the author's choice, so an infallible body
    // is not something to warn about: clippy's suggestion (`remove Result`) would make the
    // command stop compiling. The `allow` speaks only to clippy, which every other build
    // ignores.
    // `needless_pass_by_value` is the same story from the other side: clap builds the value
    // and hands it over, so a `String` is not an allocation to avoid but the argument's type.
    parsed.attrs.push(syn::parse_quote!(#[allow(
        clippy::unnecessary_wraps,
        clippy::needless_pass_by_value
    )]));

    Ok(quote! {
        #parsed

        #(#documentation)*
        /// The command's arguments, generated from the function's parameters.
        #derives
        #[facet(input = #input)]
        pub struct #leaf {
            #(#struct_fields,)*
        }

        /// What the command runs on, generated from the function's parameters.
        ///
        /// Only [`Facet::validate`] builds one: every value of it is runnable, so a caller that
        /// holds one holds something the command accepts.
        #[derive(::core::fmt::Debug, ::core::clone::Clone)]
        pub struct #input {
            #(#input_fields,)*
        }

        impl #leaf {
            /// Calls the function: one spelling of the call, and of what its parameters are.
            fn __clifacet_call(#(#helper_parameters),*)
                -> ::core::result::Result<#output, ::clifacet::Failure>
            {
                #call
            }
        }

        #implementation
    })
}

/// The impl that makes the function a command: `Handler<Cx>` with a context, `Command` without.
///
/// A command with a context is a handler and nothing else — the library's blanket impl turns
/// every `Command` into a `Handler`, so both would be two impls that disagree about which runs.
fn implementation(
    leaf: &syn::Ident,
    input: &syn::Ident,
    output: &Type,
    context: Option<&Type>,
    patterns: &[&syn::Ident],
) -> TokenStream {
    // A command with a context is a `Handler` and nothing else: the library's blanket impl
    // turns every `Command` into a `Handler`, so both would be two impls that disagree about
    // which one runs. The context type is written by the signature, so the folder's
    // `context = Cx` is what decides whether this impl agrees with the tree — and a mismatch
    // is a trait error naming both, which is the one place the two files must be in step.
    if let Some(context) = context {
        quote! {
            impl ::clifacet::Handler<#context> for #leaf {
                type Input = #input;
                type Output = #output;

                fn handle(
                    &self,
                    input: #input,
                    cx: &#context,
                ) -> ::core::result::Result<#output, ::clifacet::Failure> {
                    let #input { #(#patterns,)* } = input;
                    Self::__clifacet_call(#(#patterns,)* cx)
                }
            }
        }
    } else {
        quote! {
            impl ::clifacet::Command for #leaf {
                type Input = #input;
                type Output = #output;

                fn execute(
                    &self,
                    input: #input,
                ) -> ::core::result::Result<#output, ::clifacet::Failure> {
                    let #input { #(#patterns,)* } = input;
                    Self::__clifacet_call(#(#patterns),*)
                }
            }
        }
    }
}

/// One non-context parameter: the field it becomes, and the name it is taken apart by.
struct Argument {
    /// The parameter's name, which is the generated field's name.
    pattern: syn::Ident,
    /// The parameter's type, as written.
    ty: Type,
    /// What the generated field carries: the parameter's own attributes, with every
    /// `#[clifacet(..)]` already replaced by what it stands for.
    attributes: Vec<Attribute>,
}

impl Argument {
    /// The field of the parsed struct.
    ///
    /// Public because clap's derive, which runs where the struct is visible and not where this
    /// was written, builds the struct from its own module: a private field would make the
    /// command unparseable from anywhere but here.
    fn field(&self) -> TokenStream {
        let (pattern, ty, attributes) = (&self.pattern, &self.ty, &self.attributes);
        quote_spanned! {pattern.span()=>
            #(#attributes)*
            pub #pattern: #ty
        }
    }

    /// The matching field of the input, without the grammar: the input is what ran, not what
    /// was typed, so nothing on it is clap's.
    fn input_field(&self) -> TokenStream {
        let (pattern, ty) = (&self.pattern, &self.ty);
        quote_spanned! {pattern.span()=>
            pub #pattern: #ty
        }
    }
}

/// Reads every parameter: the context off the end, and one input field for each of the rest.
fn parameters(signature: &mut syn::Signature) -> syn::Result<(Vec<Argument>, Option<Type>)> {
    let mut arguments = Vec::new();
    let mut context: Option<Type> = None;
    let count = signature.inputs.len();

    for (index, parameter) in signature.inputs.iter_mut().enumerate() {
        let FnArg::Typed(parameter) = parameter else {
            return Err(syn::Error::new_spanned(
                parameter,
                "a function command takes no `self`: it is a free function, so there is nothing \
                 for a receiver to be",
            ));
        };
        let last = index + 1 == count;
        let ty = &*parameter.ty;

        let named = pattern_name(&parameter.pat);
        let marker = marked_as_context(&parameter.attrs)?;

        // Signatures are usually parsed without `proc_macro2`'s span locations, so a parameter
        // whose pattern is not identifiable at all is checked as if it were a value: the
        // reference check below then rejects it, and the declaration stays honest.
        let identifiable = named.is_some();
        if marker && (!identifiable || !matches!(ty, Type::Reference(_))) {
            return Err(syn::Error::new_spanned(
                &*parameter.ty,
                "`#[clifacet(context)]` marks the application's context, which the function \
                 borrows: the type is `&Cx`, not a value clap would have to build",
            ));
        }

        if let Some(reference) = context_reference(ty) {
            if identifiable && named.as_deref() != Some("cx") && !marker {
                return Err(syn::Error::new_spanned(
                    &*parameter.ty,
                    "a reference is not input: clap builds the value from the argument, and a \
                     borrowed value would have nothing to borrow. Take it by value, or — for \
                     the application's context — name the parameter `cx` and put it last",
                ));
            }
            if !last {
                return Err(syn::Error::new_spanned(
                    parameter,
                    "the context parameter comes last, so that every parameter before it is \
                     input and the command's arguments keep the order they are read in",
                ));
            }
            if context.replace(reference).is_some() {
                return Err(syn::Error::new_spanned(
                    parameter,
                    "a function command takes one context parameter: two references would \
                     disagree about which one is the application, and neither is anything clap \
                     could parse",
                ));
            }
            // The context is not parsed, so a grammar key on it would be dropped rather than
            // forwarded — the silent loss the field form refuses too. Only the marker itself
            // is expected here.
            if let Some(attribute) = parameter.attrs.iter().find(|attribute| is_ours(attribute)) {
                let entries = parse_entries(attribute).unwrap_or_else(|_| Punctuated::new());
                if entries
                    .iter()
                    .any(|entry| !entry.path().is_ident("context"))
                {
                    return Err(syn::Error::new_spanned(
                        attribute,
                        "the context parameter is not read from the command line, so it takes no \
                     `#[clifacet(..)]` keys beside `context`: declare the arguments on the \
                     parameters before it",
                    ));
                }
            }
            continue;
        }

        let Pat::Ident(pattern) = &*parameter.pat else {
            return Err(syn::Error::new_spanned(
                &parameter.pat,
                "a parameter is a name: the field is built by the parser, so a pattern here \
                 would have nothing to match",
            ));
        };
        if pattern.by_ref.is_some() || pattern.subpat.is_some() || pattern.mutability.is_some() {
            return Err(syn::Error::new_spanned(
                &parameter.pat,
                "a parameter is a plain name: `mut`, `ref` and `name @ pattern` describe a \
                 binding the parser does not make",
            ));
        }

        let attributes = forwarded_attributes(&parameter.attrs, ty)?;
        // `context` only ever appeared on the branch above; here it would be a key the field
        // table does not know.
        check_help(&attributes, &pattern.ident)?;
        arguments.push(Argument {
            pattern: pattern.ident.clone(),
            ty: ty.clone(),
            attributes,
        });
        // Nothing is left on the parameter itself: `#[clifacet(..)]` and `#[arg(..)]` are how the
        // generated field is declared, and neither is an attribute a `fn` parameter can carry.
        parameter.attrs.clear();
    }
    Ok((arguments, context))
}

/// Replaces every `#[clifacet(..)]` on a parameter with what it stands for, by the same table
/// the struct form uses, and leaves a raw `#[arg(..)]` where it is.
fn forwarded_attributes(attributes: &[Attribute], ty: &Type) -> syn::Result<Vec<Attribute>> {
    let mut forwarded = Vec::new();
    for attribute in attributes {
        if is_ours(attribute) {
            let declared = field::parse(attribute, ty)?;
            forwarded.extend(declared.attributes());
        } else {
            forwarded.push(attribute.clone());
        }
    }
    Ok(forwarded)
}

/// Refuses a flag with nothing to say about itself.
///
/// A struct field takes its description from its doc comment; a parameter cannot have one, so
/// `help` is the only place it can come from. Refusing is better than accepting, because a
/// blank line in `--help` is a defect the author would never see.
fn check_help(attributes: &[Attribute], pattern: &syn::Ident) -> syn::Result<()> {
    let mut long = None;
    let mut described = false;
    for attribute in attributes {
        if !attribute.path().is_ident("arg") {
            continue;
        }
        let Ok(entries) =
            attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        else {
            continue;
        };
        for entry in entries {
            if entry.path().is_ident("help") || entry.path().is_ident("long_help") {
                described = true;
            }
            if entry.path().is_ident("long") {
                long = Some(entry);
            }
        }
    }
    match long {
        Some(entry) if !described => Err(syn::Error::new_spanned(
            entry,
            format!(
                "`{pattern}` is a flag, and a flag needs a description: a parameter cannot have \
                 a doc comment, so add `help = \"…\"` beside `long`"
            ),
        )),
        _ => Ok(()),
    }
}

/// The checks on the signature itself, which are about the shape rather than the parameters.
fn check_signature(signature: &syn::Signature) -> syn::Result<()> {
    if let Some(parameter) = signature.generics.params.first() {
        return Err(syn::Error::new_spanned(
            parameter,
            "a function command cannot be generic: clap needs one concrete grammar, and a type \
             named here would have to be chosen by the argument list",
        ));
    }
    if let Some(asynchrony) = &signature.asyncness {
        return Err(syn::Error::new_spanned(
            asynchrony,
            "a function command cannot be `async`: nothing here awaits it, so the future it \
             returned would never run",
        ));
    }
    let ReturnType::Type(_, returned) = &signature.output else {
        return Err(syn::Error::new_spanned(
            signature,
            "a function command returns `Result<Output, ::clifacet::Failure>`, so that whoever \
             runs it knows what it produced and what went wrong: this returns nothing",
        ));
    };
    let Some((_, failure)) = result_of(returned) else {
        return Err(syn::Error::new_spanned(
            returned,
            "a function command returns `Result<Output, ::clifacet::Failure>`: the output is \
             what the application renders, and the failure is what the runner reports",
        ));
    };
    if failure
        .segments
        .last()
        .is_none_or(|segment| segment.ident != "Failure")
    {
        return Err(syn::Error::new_spanned(
            &failure,
            "a function command fails with `::clifacet::Failure`: convert inside the body with \
             `?` and `map_err`",
        ));
    }
    Ok(())
}

/// Whether the attribute is this crate's, spelled `clifacet` or `clifacet::…`.
fn is_ours(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .first()
        .is_some_and(|segment| segment.ident == field::ATTRIBUTE)
}

/// Whether the parameter carries `#[clifacet(context)]`, the marker for a context a command
/// chooses a name for.
fn marked_as_context(attributes: &[Attribute]) -> syn::Result<bool> {
    for attribute in attributes.iter().filter(|attribute| is_ours(attribute)) {
        if parse_entries(attribute)?
            .iter()
            .any(|entry| entry.path().is_ident("context"))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// One `#[clifacet(..)]`'s entries, as written.
fn parse_entries(attribute: &Attribute) -> syn::Result<Punctuated<Meta, Token![,]>> {
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(attribute.meta.require_list()?.tokens.clone())
}

/// The name a parameter is bound by, or none when it is a pattern.
fn pattern_name(pattern: &Pat) -> Option<String> {
    match pattern {
        Pat::Ident(pattern) => Some(pattern.ident.to_string()),
        _ => None,
    }
}

/// The `Cx` of a context parameter, or none for anything else.
///
/// The parameter is named `cx` — the type alone cannot say, because `&str` is a reference too
/// and the only difference between a context and a mistake would be which one clap could not
/// build. Naming it puts the declaration in the text, where a reader and this macro agree.
fn context_reference(ty: &Type) -> Option<Type> {
    let Type::Reference(reference) = ty else {
        return None;
    };
    if reference.mutability.is_some() {
        return None;
    }
    Some((*reference.elem).clone())
}

/// The `Output` and the failure path of a `Result<Output, Failure>`.
fn result_of(ty: &Type) -> Option<(Type, syn::Path)> {
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    if last.ident != "Result" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    let mut types = arguments.args.iter().filter_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    });
    let output = types.next()?.clone();
    let Type::Path(failure) = types.next()? else {
        return None;
    };
    Some((output, failure.path.clone()))
}

/// `rename` as `Rename`: the type `commands!` looks for in `rename.rs`.
fn pascal(name: &str) -> syn::Ident {
    syn::Ident::new(&name.to_upper_camel_case(), Span::call_site())
}

/// Rejects the attribute's own keys.
///
/// Every key belongs to a struct that declares its input by hand, and a function declares its
/// input by writing its parameters. Accepting a key here would be accepting a declaration that
/// does nothing.
fn check_attribute(attribute: &TokenStream) -> syn::Result<()> {
    if attribute.is_empty() {
        return Ok(());
    }
    let entries = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attribute.clone())?;
    let Some(entry) = entries.first() else {
        return Ok(());
    };
    let message = match entry.path().get_ident().map(ToString::to_string).as_deref() {
        Some("input") => {
            "a function command generates its input from its parameters, so \
                          `input = …` would be a second declaration of the same fields: drop \
                          the key and declare the fields as parameters"
        }
        Some("bridge") => {
            "a function command's body is the bridge: its parameters are the \
                           input, so `bridge = …` has nothing to add"
        }
        Some("check") => {
            "a function command validates by returning a `Failure` from its body, \
                          so `check = …` would be a second place for the same rule"
        }
        Some("collect") => {
            "a function command asks no questions, so `collect = …` has nothing \
                            to call: write the struct form where a command asks, and the \
                            function form where it does not"
        }
        Some(key) => {
            return Err(syn::Error::new_spanned(
                entry,
                format!(
                    "`{key}` is not a key of `#[clifacet::command]` on a function: a function \
                     command takes none, because the parameters are the declaration"
                ),
            ));
        }
        None => {
            return Err(syn::Error::new_spanned(
                entry,
                "`#[clifacet::command]` keys are plain names, and a function command takes none",
            ));
        }
    };
    Err(syn::Error::new_spanned(entry, message))
}
