//! `clifacet::commands!(list, show, new)`: register a command folder in one line (CLI-33).
//!
//! A tree is declared where its leaves live (decision-14), but the registration around it
//! is mechanical: one `mod` line, one `pub use` and one enum variant per leaf, a second
//! variant in the request, and a root struct. This macro writes all of that from a list of
//! module names, so a folder holds exactly one registration line:
//!
//! ```ignore
//! clifacet::commands!(list, show, new);
//! ```
//!
//! Nothing is discovered: the list *is* the registry, checked by the compiler. Every name
//! it generates is fixed, so a reader knows what a folder contains without opening it:
//!
//! | Item | What it is |
//! | --- | --- |
//! | `mod <name>;` and `pub use <name>::<Name>;` | the leaf struct, which is its own command |
//! | `enum Commands` | the tree: one `Name(<name>::Name)` variant per module |
//! | `enum Request` | what the user asked for, validated: one variant per leaf input |
//! | `struct Input` | the root's validated input: `command: Option<Request>` |
//! | `struct Cli` | the root: the subcommand, [`Globals`](clifacet::Globals), the extras |
//!
//! The tree is the same `#[clifacet::subcommands]` this crate's attribute generates, so
//! dispatch stays compile-time and a request that leaves out a variant does not compile.
//! The file stem is the command name in kebab case and the struct is its Pascal case
//! form, as `clap` itself would name them — `search_history` is `SearchHistory` named
//! `search-history` — and a leaf may still rename itself with `#[command(name = "..")]`.
//!
//! Keys, after the module names:
//!
//! - `globals = Type`: the application's own global options, a `clap::Args` type,
//!   flattened beside `Globals` and kept out of the input with `#[clifacet(skip)]`;
//! - `output = Type`: what every leaf's output converts into, for a tree that runs;
//! - `context = Type`: the application's context, threaded to the chosen leaf (CLI-47);
//! - `planned`: the tree delegates `plan` instead of running;
//! - `name = "…"`, `about = "…"`: the root's name and description, otherwise taken from
//!   Cargo at compile time;
//! - `version_off`: no `--version`, for a root that forwards version queries elsewhere.

use heck::ToUpperCamelCase as _;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{ParseStream, Parser as _};
use syn::spanned::Spanned as _;
use syn::{Ident, LitStr, Token, Type};

use crate::subcommands;

/// The keys `commands!(..)` takes once its module names are read, for messages.
const SUPPORTED_KEYS: &str = "`globals = Type`, `output = Type`, `context = Type`, `planned`, \
                              `name = \"…\"`, `about = \"…\"`, `long_about = \"…\"`, \
                              `version_off`, `root = { \"field\": Type }`, `request = Ident`";

/// One call: the modules it registers, and the keys it sets.
struct Declared {
    modules: Vec<syn::Path>,
    globals: Option<Type>,
    output: Option<Type>,
    context: Option<Type>,
    planned: bool,
    name: Option<LitStr>,
    about: Option<LitStr>,
    long_about: Option<LitStr>,
    version_off: bool,
    /// What the generated request enum is called; `Request` unless a name is given.
    request: Ident,
    /// The root's own fields, declared the way a `#[clifacet::command]` struct declares
    /// them and folded into the generated input by name: `root = {"json": bool}`.
    root: Vec<(Ident, Type)>,
}

impl Default for Declared {
    fn default() -> Self {
        Self {
            modules: Vec::new(),
            globals: None,
            output: None,
            context: None,
            planned: false,
            name: None,
            about: None,
            long_about: None,
            version_off: false,
            root: Vec::new(),
            request: Ident::new("Request", Span::call_site()),
        }
    }
}

impl Declared {
    /// Rejects what a valid call cannot mean: no command, a command twice, `planned`
    /// with a context.
    fn finish(self) -> syn::Result<Self> {
        if self.modules.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "`clifacet::commands!` needs at least one command: name the modules that hold \
                 them, `clifacet::commands!(list, show, new)` — one file per command, and this \
                 list is the only registry",
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for module in &self.modules {
            if !seen.insert(quote!(#module).to_string()) {
                return Err(syn::Error::new(
                    module.span(),
                    format!(
                        "`{}` is listed twice; one module, one command — drop the second",
                        quote!(#module)
                    ),
                ));
            }
        }
        if self.context.is_some() && self.planned {
            return Err(syn::Error::new(
                Span::call_site(),
                "`context` and `planned` cannot both be declared: with a context the tree \
                 previews through each leaf's own `Handler::preview`, so `planned` has \
                 nothing to add — drop it",
            ));
        }
        Ok(self)
    }

    /// The root's own fields as the generated input declares them: copied by name.
    fn root_fields(&self) -> Vec<TokenStream> {
        self.root
            .iter()
            .map(|(field, ty)| {
                quote! {
                    /// A field of the root, copied as declared.
                    pub #field: #ty,
                }
            })
            .collect()
    }

    /// The root's own flags: declared here, so the input copies them by name, and global,
    /// so a subcommand may be written before or after them.
    fn root_flags(&self) -> Vec<TokenStream> {
        self.root
            .iter()
            .map(|(field, ty)| {
                quote! {
                    /// A field of the root, declared by `commands!`: a global long flag, in
                    /// the input under the same name.
                    #[clifacet(long)]
                    #[arg(global = true)]
                    pub #field: #ty,
                }
            })
            .collect()
    }

    /// `long_about = "…"` for clap, or nothing.
    fn long_about_attr(&self) -> Option<TokenStream> {
        self.long_about
            .as_ref()
            .map(|long_about| quote!(long_about = #long_about,))
    }

    /// The leaf struct each module holds, named in Pascal case.
    ///
    /// A module may nest (`db::migrate`), and then the leaf is `Migrate` in the module of
    /// the same name: the path is the module's, the struct is its last segment's.
    fn leaves(&self) -> Vec<LeafPath> {
        self.modules
            .iter()
            .map(|module| {
                let last = module.segments.last().map_or_else(
                    || Ident::new("command", Span::call_site()),
                    |segment| segment.ident.clone(),
                );
                LeafPath {
                    path: module.clone(),
                    leaf: pascal(&last),
                    variant: pascal(&last),
                }
            })
            .collect()
    }
}

/// One registered command: where it lives, its struct, and its variant in both enums.
struct LeafPath {
    path: syn::Path,
    leaf: Ident,
    variant: Ident,
}

/// `list` as `List`, `search_history` as `SearchHistory`.
fn pascal(module: &Ident) -> Ident {
    // `Span::call_site()` rather than the module's own span: the tree's generated arms
    // span their bindings at the leaf type, and a span from inside this expansion would
    // put those bindings in this macro's hygiene context instead of the caller's.
    Ident::new(&module.to_string().to_upper_camel_case(), Span::call_site())
}

/// Expands one `commands!(..)` call into the registration, or an error saying what to write.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    match parse(input).and_then(|declared| tokens(&declared)) {
        Ok(tokens) => tokens,
        Err(error) => error.into_compile_error(),
    }
}

/// Reads the module list and the keys after it.
fn parse(input: TokenStream) -> syn::Result<Declared> {
    let parser = |stream: ParseStream<'_>| -> syn::Result<Declared> {
        let mut declared = Declared::default();
        let mut first = true;
        while !stream.is_empty() {
            if !first {
                stream.parse::<Token![,]>()?;
                if stream.is_empty() {
                    break;
                }
            }
            first = false;
            if stream.peek(Token![,]) {
                continue;
            }
            declaration(stream, &mut declared)?;
        }
        declared.finish()
    };
    parser.parse2(input)
}

/// Reads one entry: a module to register, or a key and its value.
fn declaration(stream: ParseStream<'_>, declared: &mut Declared) -> syn::Result<()> {
    // A module name, or a nested path (`db::migrate`) that reaches into a group, unless
    // the first name is one this macro takes as a key.
    let first: Ident = stream.parse()?;
    let known = matches!(
        first.to_string().as_str(),
        "globals"
            | "output"
            | "context"
            | "planned"
            | "name"
            | "about"
            | "long_about"
            | "version_off"
            | "root"
            | "request"
    );
    if !known {
        let mut module: syn::Path = syn::Path::from(first.clone());
        while stream.peek(Token![::]) {
            stream.parse::<Token![::]>()?;
            let segment: Ident = stream.parse()?;
            module.segments.push(segment.into());
        }
        declared.modules.push(module);
        // A name that is no module is a key this macro does not take, and saying so now
        // beats "file not found for module".
        if stream.peek(Token![=]) {
            return Err(unknown_key(&first.to_string(), first.span()));
        }
        return Ok(());
    }
    let (name, key) = (first.to_string(), first);
    if stream.peek(Token![=]) {
        stream.parse::<Token![=]>()?;
        return match name.as_str() {
            "globals" => {
                declared.globals = Some(stream.parse()?);
                Ok(())
            }
            "output" => {
                declared.output = Some(stream.parse()?);
                Ok(())
            }
            "context" => {
                declared.context = Some(stream.parse()?);
                Ok(())
            }
            "name" => {
                declared.name = Some(stream.parse()?);
                Ok(())
            }
            "about" => {
                declared.about = Some(stream.parse()?);
                Ok(())
            }
            "long_about" => {
                declared.long_about = Some(stream.parse()?);
                Ok(())
            }
            "root" => {
                declared.root = root_fields(stream)?;
                Ok(())
            }
            "request" => {
                declared.request = stream.parse()?;
                Ok(())
            }
            known @ ("planned" | "version_off") => Err(syn::Error::new(
                key.span(),
                format!("`{known}` takes no value: write `{known}` on its own"),
            )),
            _ => Err(unknown_key(&name, key.span())),
        };
    }
    match name.as_str() {
        "planned" => declared.planned = true,
        "version_off" => declared.version_off = true,
        _ => {
            return Err(syn::Error::new(
                key.span(),
                format!("`{name}` takes a value, for example `{name} = MyType`"),
            ));
        }
    }
    Ok(())
}

/// Reads `{ "json": bool, "verbose": Option<String> }` into field declarations.
///
/// Only what the generated input copies can be declared here — a name, its type, and
/// either `#[clifacet(skip)]`-like absence or presence in the input. Anything else about
/// the root's own flags is written on the field this returns, in the root's own idiom.
fn root_fields(stream: ParseStream<'_>) -> syn::Result<Vec<(Ident, Type)>> {
    if !stream.peek(syn::token::Brace) {
        let found: TokenStream = stream.parse()?;
        return Err(syn::Error::new_spanned(
            found,
            "`root` takes the root's own input fields in braces, for example \
             `root = { \"json\": bool }` — a name for the input, its type for the value",
        ));
    }
    let content;
    syn::braced!(content in stream);
    let fields: syn::punctuated::Punctuated<RootField, Token![,]> =
        content.parse_terminated(|stream: ParseStream<'_>| stream.parse(), Token![,])?;
    Ok(fields
        .into_iter()
        .map(|field| (field.name, field.ty))
        .collect())
}

/// One `"name": Type` pair inside `root = { .. }`.
struct RootField {
    name: Ident,
    ty: Type,
}

impl syn::parse::Parse for RootField {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let field: LitStr = input.parse()?;
        let name = Ident::new(&field.value(), field.span());
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        Ok(Self { name, ty })
    }
}

fn unknown_key(key: &str, span: Span) -> syn::Error {
    syn::Error::new(
        span,
        format!(
            "`{key}` is not a key of `clifacet::commands!(..)`; supported keys: {SUPPORTED_KEYS}"
        ),
    )
}

/// The registration: modules, re-exports, the tree, the request, the input and the root.
fn tokens(declared: &Declared) -> syn::Result<TokenStream> {
    let modules = &declared.modules;
    let leaves = declared.leaves();
    let variants = leaves.iter().map(|leaf| {
        let (variant, name) = (&leaf.variant, &leaf.leaf);
        quote!(#variant(#name))
    });
    let requests = leaves.iter().map(|leaf| {
        let (variant, path, name) = (&leaf.variant, &leaf.path, &leaf.leaf);
        quote!(#variant(<#path::#name as ::clifacet::Facet>::Input))
    });
    // Re-exported, so the leaf, its payload and its `forward` are reachable from the
    // folder's own modules and from an application that names them: `pub(crate)` where
    // the caller's visibility allows it, which is what makes one call work in an
    // application and in a test.
    let reexports = leaves.iter().map(|leaf| {
        let (path, name) = (&leaf.path, &leaf.leaf);
        quote!(pub(crate) use #path::#name;)
    });

    // The tree's attribute is the one `#[clifacet::subcommands]` parses, forwarded whole:
    // this macro never re-implements the tree, it only says what to declare.
    let request = &declared.request;
    let mut attribute = quote!(input = #request);
    if let Some(output) = &declared.output {
        attribute.extend(quote!(, output = #output));
    }
    if declared.planned {
        attribute.extend(quote!(, planned));
    }
    if let Some(context) = &declared.context {
        attribute.extend(quote!(, context = #context));
    }
    // The tree is expanded here, with the attribute's own parser: forwarding the
    // attribute as tokens would expand it in the attribute's context, where the leaf types
    // this macro named are invisible ("cannot find value `leaf`").
    let tree = subcommands::expand(
        attribute,
        quote! {
            #[derive(Clone)]
            pub enum Commands {
                #(#variants,)*
            }
        },
    )?;

    let root_fields = declared.root_fields();
    let root_flags = declared.root_flags();

    let name = declared.name.as_ref().map_or_else(
        || quote!(::core::env!("CARGO_PKG_NAME")),
        |name| quote!(#name),
    );
    let about = declared.about.as_ref().map_or_else(
        || quote!(::core::env!("CARGO_PKG_DESCRIPTION")),
        |about| quote!(#about),
    );
    let long_about = declared.long_about_attr();
    let version = if declared.version_off {
        quote!()
    } else {
        quote!(version,)
    };
    let globals = declared.globals.as_ref().map(|ty| {
        quote! {
            /// The application's own global options.
            #[command(flatten)]
            #[clifacet(skip)]
            pub globals_of_the_application: #ty,
        }
    });

    // The root runs what its tree runs: the same kind, delegating to the chosen leaf, so
    // an application's `main` hands the runner one command and a dry run reaches the leaf.
    let root_run = root(declared);

    // The order is the order the macro is written in, so a leaf's variant sits where its
    // module is listed and the compiler's messages follow the file.
    Ok(quote! {
        #(mod #modules;)*
        #(#reexports)*

        /// What the user asked for, validated: one variant per command.
        // Prints and clones; equality is the payload's to offer, so it is not derived here.
        #[derive(Debug, Clone)]
        pub enum #request {
            #(#requests,)*
        }

        /// The tree: one variant per command, parsed by clap and dispatched by the compiler.
        #tree

        /// The root's validated input: what to run, and the root's own fields beside it.
        #[derive(Debug, Clone)]
        pub struct Input {
            /// What to run; none means the application's own default.
            pub command: Option<#request>,
            #(#root_fields)*
        }



        /// The root command: the subcommand, the globals every application gets, and the
        /// application's own.
        // `Clone`, because the runner parses once and may run the command again: the
        // root holds its leaves, so they are cloned with it.
        // `Clone`, because a run parses once and may run again; nothing else, because the
        // root holds the leaves and a leaf is whatever it needs to be.
        #[clifacet::command(input = Input)]
        #[derive(Clone)]
        #[command(name = #name, #version about = #about, #long_about)]
        pub struct Cli {
            /// The subcommand; none means the application's own default.
            // Spelled `Option`, not `::core::option::Option`: clap's derive recognizes
            // an optional subcommand by that exact path, and treats any other as required.
            #[clifacet(subcommands)]
            pub command: Option<Commands>,

            #[command(flatten)]
            #[clifacet(skip)]
            pub globals: ::clifacet::Globals,

            #(#root_flags)*

            #globals
        }

        #root_run
    })
}

/// The root's dispatch: it runs what its tree runs, so `main` hands the runner one command.
///
/// The bodies are written out rather than delegated to a helper trait: a root with a
/// context has a context to pass, and one without has none, and the two signatures differ
/// in exactly that.
fn root(declared: &Declared) -> TokenStream {
    let fallback = quote! {
        ::core::result::Result::Err(::clifacet::Failure::validation("name the command to run"))
    };
    if declared.planned {
        // `planned` is also a `Command` (the library's blanket impl), so it is checked
        // first: delegating `plan` is what makes a dry run reach the leaf.
        return quote! {
            impl ::clifacet::command::Planned for Cli {
                type Input = Input;
                type Output = <Commands as ::clifacet::command::Planned>::Output;

                fn plan(
                    &self,
                    input: Input,
                ) -> ::core::result::Result<
                    ::clifacet::command::Plan<'_, Self::Output>,
                    ::clifacet::Failure,
                > {
                    match (&self.command, input.command) {
                        (Some(tree), Some(request)) => {
                            <Commands as ::clifacet::command::Planned>::plan(tree, request)
                        }
                        _ => #fallback,
                    }
                }
            }
        };
    }
    match (&declared.output, &declared.context) {
        (_, Some(context)) => quote! {
            impl ::clifacet::Handler<#context> for Cli {
                type Input = Input;
                type Output = <Commands as ::clifacet::Handler<#context>>::Output;

                fn handle(
                    &self,
                    input: Input,
                    cx: &#context,
                ) -> ::core::result::Result<Self::Output, ::clifacet::Failure> {
                    match (&self.command, input.command) {
                        (Some(tree), Some(request)) => {
                            <Commands as ::clifacet::Handler<#context>>::handle(tree, request, cx)
                        }
                        _ => #fallback,
                    }
                }

                fn preview<'__clifacet>(
                    &'__clifacet self,
                    input: Input,
                    cx: &'__clifacet #context,
                ) -> ::core::result::Result<
                    ::clifacet::command::Plan<'__clifacet, Self::Output>,
                    ::clifacet::Failure,
                > {
                    match (&self.command, input.command) {
                        (Some(tree), Some(request)) => <Commands as ::clifacet::Handler<
                            #context,
                        >>::preview(tree, request, cx),
                        _ => #fallback,
                    }
                }
            }
        },
        (Some(output), None) => quote! {
            impl ::clifacet::Command for Cli {
                type Input = Input;
                type Output = #output;

                fn execute(
                    &self,
                    input: Input,
                ) -> ::core::result::Result<Self::Output, ::clifacet::Failure> {
                    match (&self.command, input.command) {
                        (Some(tree), Some(request)) => {
                            <Commands as ::clifacet::Command>::execute(tree, request)
                        }
                        _ => #fallback,
                    }
                }
            }
        },
        (None, None) => quote!(),
    }
}
