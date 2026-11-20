//! `#[clifacet::app(...)]`: the declarative application controller macro (decision-24).

use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, Ident, Lit, LitStr, Meta, Path, PathArguments,
    Token, Type,
};

enum VersionArg {
    Default,
    Specified(LitStr),
}

#[derive(Clone)]
enum SkillConfig {
    Default,
    Renderer(Path),
    Template(LitStr),
    TemplatePath(LitStr),
}

#[derive(Default)]
struct AppArgs {
    name: Option<LitStr>,
    version: Option<VersionArg>,
    about: Option<LitStr>,
    long_about: Option<LitStr>,
    author: Option<LitStr>,
    before_run: Option<Path>,
    after_success: Option<Path>,
    after_failure: Option<Path>,
    input: Option<Type>,
    commands: Option<Option<Ident>>,
    schema: Option<bool>,
    skill: Option<SkillConfig>,
}

struct ContextFieldInfo {
    ident: Ident,
    ty: Type,
    custom_factory: Option<Path>,
}

struct SubcommandFieldInfo {
    ident: Ident,
    is_option: bool,
}

pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut parsed = syn::parse2::<DeriveInput>(item)?;
    let Data::Struct(data_struct) = &mut parsed.data else {
        return Err(syn::Error::new_spanned(
            &parsed,
            "#[clifacet::app] can only be applied to a struct with named fields",
        ));
    };
    let Fields::Named(fields_named) = &mut data_struct.fields else {
        return Err(syn::Error::new_spanned(
            &parsed,
            "#[clifacet::app] can only be applied to a struct with named fields",
        ));
    };

    let app_args = parse_app_args(attribute)?;

    let mut context_field: Option<ContextFieldInfo> = None;
    let mut subcommand_field: Option<SubcommandFieldInfo> = None;

    for field in &mut fields_named.named {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(&field, "expected named field"))?;

        let mut is_context = false;
        let mut custom_factory: Option<Path> = None;
        let mut is_subcommand = false;
        let mut remaining_attrs = Vec::new();

        for attr in field.attrs.drain(..) {
            if attr.path().is_ident("context") {
                is_context = true;
            } else if attr.path().is_ident("subcommand") {
                is_subcommand = true;
            } else if attr.path().is_ident(crate::field::ATTRIBUTE) {
                let (has_ctx, factory, has_subcmd, other_metas) = process_clifacet_attr(&attr)?;
                if has_ctx {
                    is_context = true;
                    custom_factory = factory;
                }
                if has_subcmd {
                    is_subcommand = true;
                }
                if !other_metas.is_empty() {
                    let new_attr: Attribute = syn::parse_quote!(#[clifacet(#(#other_metas),*)]);
                    remaining_attrs.push(new_attr);
                }
            } else {
                remaining_attrs.push(attr);
            }
        }

        if is_context {
            if context_field.is_some() {
                return Err(syn::Error::new_spanned(
                    &field,
                    "only one field can be marked with `#[clifacet(context)]`",
                ));
            }
            context_field = Some(ContextFieldInfo {
                ident: field_ident.clone(),
                ty: field.ty.clone(),
                custom_factory,
            });
            let orig_ty = &field.ty;
            field.ty = syn::parse_quote!(::clifacet::ContextCell<#orig_ty>);
            remaining_attrs.push(syn::parse_quote!(#[arg(skip)]));
        }

        if is_subcommand {
            let is_option = is_option_type(&field.ty);
            subcommand_field = Some(SubcommandFieldInfo {
                ident: field_ident,
                is_option,
            });
            remaining_attrs.push(syn::parse_quote!(#[command(subcommand)]));
        }

        field.attrs = remaining_attrs;
    }

    if let Some(custom_commands_name) = &app_args.commands {
        if subcommand_field.is_none() {
            let enum_ident = custom_commands_name
                .clone()
                .unwrap_or_else(|| quote::format_ident!("{}Commands", parsed.ident));
            let cmd_field_ident = quote::format_ident!("command");
            subcommand_field = Some(SubcommandFieldInfo {
                ident: cmd_field_ident.clone(),
                is_option: true,
            });
            fields_named.named.push(syn::parse_quote! {
                #[command(subcommand)]
                pub #cmd_field_ident: Option<#enum_ident>
            });
        }
    }

    let has_globals = fields_named
        .named
        .iter()
        .any(|f| f.ident.as_ref().is_some_and(|i| i == "globals"));

    // Forward any remaining field attributes through crate::field
    crate::command::forward_field_attributes(&mut parsed)?;

    // Build clap #[command(...)] attribute
    let mut command_args = Vec::new();
    if let Some(name) = &app_args.name {
        command_args.push(quote!(name = #name));
    }
    if let Some(version) = &app_args.version {
        match version {
            VersionArg::Default => command_args.push(quote!(version = env!("CARGO_PKG_VERSION"))),
            VersionArg::Specified(v) => command_args.push(quote!(version = #v)),
        }
    }
    if let Some(about) = &app_args.about {
        command_args.push(quote!(about = #about));
    }
    if let Some(long_about) = &app_args.long_about {
        command_args.push(quote!(long_about = #long_about));
    }
    if let Some(author) = &app_args.author {
        command_args.push(quote!(author = #author));
    }

    let command_attr = if command_args.is_empty() {
        quote!()
    } else {
        quote!(#[command(#(#command_args),*)])
    };

    let name = &parsed.ident;
    let (impl_generics, type_generics, where_clause) = parsed.generics.split_for_impl();

    let init_from_parent = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #[doc(hidden)]
            pub fn init_from_parent<P>(&self, _parent: &P) {}
        }
    };

    let (context_type, init_body) = if let Some(cx) = &context_field {
        let cx_ident = &cx.ident;
        let cx_ty = &cx.ty;
        let init = if let Some(factory) = &cx.custom_factory {
            quote! {
                let cx = #factory(self)?;
                let _ = self.#cx_ident.set(::std::clone::Clone::clone(&cx));
                ::core::result::Result::Ok(cx)
            }
        } else {
            quote! {
                use ::clifacet::cli::auto_wire::{AutoWireDefault as _, AutoWireFromCli as _};
                let cx = (&&::clifacet::cli::auto_wire::AutoWire::<#cx_ty>(::std::marker::PhantomData)).init_ctx(self)?;
                let _ = self.#cx_ident.set(::std::clone::Clone::clone(&cx));
                ::core::result::Result::Ok(cx)
            }
        };
        (quote!(#cx_ty), init)
    } else {
        (quote!(()), quote!(::core::result::Result::Ok(())))
    };

    let before_run_impl = app_args.before_run.as_ref().map(|path| {
        quote! {
            fn before_run(&self, cmd: &str, cx: &Self::Context) -> ::core::result::Result<(), ::clifacet::Failure> {
                #path(self, cmd, cx)
            }
        }
    });
    let after_success_impl = app_args.after_success.as_ref().map(|path| {
        quote! {
            fn after_success(&self, cmd: &str, cx: &Self::Context) -> ::core::result::Result<(), ::clifacet::Failure> {
                #path(self, cmd, cx)
            }
        }
    });
    let after_failure_impl = app_args.after_failure.as_ref().map(|path| {
        quote! {
            fn after_failure(&self, cmd: &str, err: &::clifacet::Failure, cx: ::core::option::Option<&Self::Context>) {
                #path(self, cmd, err, cx);
            }
        }
    });

    let cli_impl = quote! {
        impl #impl_generics ::clifacet::Cli for #name #type_generics #where_clause {
            type Context = #context_type;

            fn init_context(&self) -> ::core::result::Result<Self::Context, ::clifacet::Failure> {
                #init_body
            }

            #before_run_impl
            #after_success_impl
            #after_failure_impl
        }
    };

    let input_type = app_args
        .input
        .as_ref()
        .map_or_else(|| quote!(Self), |ty| quote!(#ty));
    let facet_validate = if app_args.input.is_some() {
        quote!(::core::convert::TryFrom::try_from(self))
    } else {
        quote!(::core::result::Result::Ok(self))
    };

    let facet_impl = quote! {
        impl #impl_generics ::clifacet::Facet for #name #type_generics #where_clause {
            type Input = #input_type;

            fn command() -> ::clap::Command {
                <Self as ::clap::CommandFactory>::command()
            }

            fn validate(self) -> ::core::result::Result<Self::Input, ::clifacet::Failure> {
                #facet_validate
            }
        }
    };

    let collect_impl = if let Some(subcmd) = &subcommand_field {
        let subcmd_id = &subcmd.ident;
        let collect_tree_call = if subcmd.is_option {
            quote!(::clifacet::facet::__derive::collect_optional_tree(self.#subcmd_id, session)?)
        } else {
            quote!(::clifacet::facet::__derive::collect_tree(self.#subcmd_id, session)?)
        };
        quote! {
            impl #impl_generics ::clifacet::prompt::Collect for #name #type_generics #where_clause {
                fn collect(
                    mut self,
                    session: &mut ::clifacet::prompt::Session,
                ) -> ::core::result::Result<Self, ::clifacet::Failure> {
                    self.#subcmd_id = #collect_tree_call;
                    ::core::result::Result::Ok(self)
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics ::clifacet::prompt::Collect for #name #type_generics #where_clause {
                fn collect(
                    self,
                    _session: &mut ::clifacet::prompt::Session,
                ) -> ::core::result::Result<Self, ::clifacet::Failure> {
                    ::core::result::Result::Ok(self)
                }
            }
        }
    };

    let is_json_expr = if has_globals {
        quote!(self.globals.json)
    } else {
        quote!(false)
    };

    let json_inherent_impl = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #[doc(hidden)]
            pub fn __clifacet_is_json(&self) -> bool {
                #is_json_expr
            }
        }
    };

    let schema_inherent_impl = if app_args.schema == Some(false) {
        quote!()
    } else {
        quote! {
            #[cfg(feature = "schema")]
            impl #impl_generics #name #type_generics #where_clause {
                /// Returns machine-readable metadata describing this CLI command hierarchy.
                pub fn cli_metadata() -> ::clifacet::schema::CliMetadata {
                    ::clifacet::schema::CliMetadata::from_command(&<Self as ::clap::CommandFactory>::command())
                }

                /// Emits the JSON Schema for this CLI hierarchy as a formatted string.
                pub fn cli_schema_json() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                    let meta = Self::cli_metadata();
                    ::clifacet::schema::to_json_schema_pretty(&meta)
                        .map_err(|e| ::clifacet::Failure::execution(format!("cannot generate schema: {e}")))
                }
            }
        }
    };

    let skill_inherent_impl = match &app_args.skill {
        Some(SkillConfig::Renderer(renderer_path)) => {
            quote! {
                #[cfg(feature = "skill")]
                impl #impl_generics #name #type_generics #where_clause {
                    /// Renders the CLI skill documentation using the configured [`#renderer_path`] renderer.
                    pub fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = <#renderer_path as ::core::default::Default>::default();
                        Self::render_skill_with(&renderer)
                    }

                    /// Returns a new instance of the configured skill renderer.
                    pub fn skill_renderer() -> #renderer_path {
                        <#renderer_path as ::core::default::Default>::default()
                    }

                    /// Writes or checks the agent skill documentation according to the given options.
                    pub fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let rendered = Self::render_skill()?;
                        let meta = Self::cli_metadata();
                        ::clifacet::skill::execute_write_skill(&meta.name, &rendered, options)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template string.
                    pub fn render_skill_with_template(
                        template: &str,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::with_template(template);
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template file path.
                    pub fn render_skill_with_template_file(
                        path: impl ::core::convert::AsRef<::std::path::Path>,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::from_file(path)?;
                        Self::render_skill_with(&renderer)
                    }

                    /// Renders the CLI skill documentation using the provided [`SkillRenderer`].
                    pub fn render_skill_with<R: ::clifacet::skill::SkillRenderer>(
                        renderer: &R,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let meta = Self::cli_metadata();
                        renderer.render(&meta)
                    }
                }

                #[cfg(feature = "skill")]
                impl #impl_generics ::clifacet::skill::SkillProvider for #name #type_generics #where_clause {
                    type Renderer = #renderer_path;

                    fn skill_renderer() -> Self::Renderer {
                        <#renderer_path as ::core::default::Default>::default()
                    }

                    fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::render_skill()
                    }

                    fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::write_skill(options)
                    }
                }
            }
        }
        Some(SkillConfig::Template(tmpl_lit)) => {
            quote! {
                #[cfg(feature = "skill")]
                impl #impl_generics #name #type_generics #where_clause {
                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using the configured template.
                    pub fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = Self::skill_renderer();
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Returns a new instance of the configured skill renderer.
                    pub fn skill_renderer() -> ::clifacet::skill::MiniJinjaSkillRenderer {
                        ::clifacet::skill::MiniJinjaSkillRenderer::with_template(#tmpl_lit)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Writes or checks the agent skill documentation according to the given options.
                    pub fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let rendered = Self::render_skill()?;
                        let meta = Self::cli_metadata();
                        ::clifacet::skill::execute_write_skill(&meta.name, &rendered, options)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template string.
                    pub fn render_skill_with_template(
                        template: &str,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::with_template(template);
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template file path.
                    pub fn render_skill_with_template_file(
                        path: impl ::core::convert::AsRef<::std::path::Path>,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::from_file(path)?;
                        Self::render_skill_with(&renderer)
                    }

                    /// Renders the CLI skill documentation using the provided [`SkillRenderer`].
                    pub fn render_skill_with<R: ::clifacet::skill::SkillRenderer>(
                        renderer: &R,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let meta = Self::cli_metadata();
                        renderer.render(&meta)
                    }
                }

                #[cfg(all(feature = "skill", feature = "skill-minijinja"))]
                impl #impl_generics ::clifacet::skill::SkillProvider for #name #type_generics #where_clause {
                    type Renderer = ::clifacet::skill::MiniJinjaSkillRenderer;

                    fn skill_renderer() -> Self::Renderer {
                        <#name #type_generics>::skill_renderer()
                    }

                    fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::render_skill()
                    }

                    fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::write_skill(options)
                    }
                }
            }
        }
        Some(SkillConfig::TemplatePath(path_lit)) => {
            quote! {
                #[cfg(feature = "skill")]
                impl #impl_generics #name #type_generics #where_clause {
                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using the template file.
                    pub fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = Self::skill_renderer();
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Returns a new instance of the configured skill renderer.
                    pub fn skill_renderer() -> ::clifacet::skill::MiniJinjaSkillRenderer {
                        ::clifacet::skill::MiniJinjaSkillRenderer::with_template(::core::include_str!(#path_lit))
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Writes or checks the agent skill documentation according to the given options.
                    pub fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let rendered = Self::render_skill()?;
                        let meta = Self::cli_metadata();
                        ::clifacet::skill::execute_write_skill(&meta.name, &rendered, options)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template string.
                    pub fn render_skill_with_template(
                        template: &str,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::with_template(template);
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template file path.
                    pub fn render_skill_with_template_file(
                        path: impl ::core::convert::AsRef<::std::path::Path>,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::from_file(path)?;
                        Self::render_skill_with(&renderer)
                    }

                    /// Renders the CLI skill documentation using the provided [`SkillRenderer`].
                    pub fn render_skill_with<R: ::clifacet::skill::SkillRenderer>(
                        renderer: &R,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let meta = Self::cli_metadata();
                        renderer.render(&meta)
                    }
                }

                #[cfg(all(feature = "skill", feature = "skill-minijinja"))]
                impl #impl_generics ::clifacet::skill::SkillProvider for #name #type_generics #where_clause {
                    type Renderer = ::clifacet::skill::MiniJinjaSkillRenderer;

                    fn skill_renderer() -> Self::Renderer {
                        <#name #type_generics>::skill_renderer()
                    }

                    fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::render_skill()
                    }

                    fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::write_skill(options)
                    }
                }
            }
        }
        Some(SkillConfig::Default) => {
            quote! {
                #[cfg(feature = "skill")]
                impl #impl_generics #name #type_generics #where_clause {
                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using the default [`MiniJinjaSkillRenderer`].
                    pub fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::new();
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Returns a new instance of the default skill renderer.
                    pub fn skill_renderer() -> ::clifacet::skill::MiniJinjaSkillRenderer {
                        ::clifacet::skill::MiniJinjaSkillRenderer::new()
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Writes or checks the agent skill documentation according to the given options.
                    pub fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let rendered = Self::render_skill()?;
                        let meta = Self::cli_metadata();
                        ::clifacet::skill::execute_write_skill(&meta.name, &rendered, options)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template string.
                    pub fn render_skill_with_template(
                        template: &str,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::with_template(template);
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template file path.
                    pub fn render_skill_with_template_file(
                        path: impl ::core::convert::AsRef<::std::path::Path>,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::from_file(path)?;
                        Self::render_skill_with(&renderer)
                    }

                    /// Renders the CLI skill documentation using the provided [`SkillRenderer`].
                    pub fn render_skill_with<R: ::clifacet::skill::SkillRenderer>(
                        renderer: &R,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let meta = Self::cli_metadata();
                        renderer.render(&meta)
                    }
                }

                #[cfg(all(feature = "skill", feature = "skill-minijinja"))]
                impl #impl_generics ::clifacet::skill::SkillProvider for #name #type_generics #where_clause {
                    type Renderer = ::clifacet::skill::MiniJinjaSkillRenderer;

                    fn skill_renderer() -> Self::Renderer {
                        ::clifacet::skill::MiniJinjaSkillRenderer::new()
                    }

                    fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::render_skill()
                    }

                    fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::write_skill(options)
                    }
                }
            }
        }
        None => {
            quote! {
                #[cfg(feature = "skill")]
                impl #impl_generics #name #type_generics #where_clause {
                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using the default MiniJinja template.
                    pub fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::new();
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Returns a new instance of the default skill renderer.
                    pub fn skill_renderer() -> ::clifacet::skill::MiniJinjaSkillRenderer {
                        ::clifacet::skill::MiniJinjaSkillRenderer::new()
                    }

                    /// Writes or checks the agent skill documentation according to the given options.
                    pub fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        #[cfg(feature = "skill-minijinja")]
                        {
                            use ::clifacet::skill::SkillRenderer as _;
                            let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::new();
                            let meta = Self::cli_metadata();
                            let rendered = renderer.render(&meta)?;
                            ::clifacet::skill::execute_write_skill(&meta.name, &rendered, options)
                        }
                        #[cfg(not(feature = "skill-minijinja"))]
                        {
                            let _ = options;
                            ::core::result::Result::Err(::clifacet::Failure::execution(
                                "no skill renderer configured (enable `skill-minijinja` or configure `skill = Renderer`)",
                            ))
                        }
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template string.
                    pub fn render_skill_with_template(
                        template: &str,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::with_template(template);
                        Self::render_skill_with(&renderer)
                    }

                    #[cfg(feature = "skill-minijinja")]
                    /// Renders the CLI skill documentation using a custom template file path.
                    pub fn render_skill_with_template_file(
                        path: impl ::core::convert::AsRef<::std::path::Path>,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let renderer = ::clifacet::skill::MiniJinjaSkillRenderer::from_file(path)?;
                        Self::render_skill_with(&renderer)
                    }

                    /// Renders the CLI skill documentation using the provided [`SkillRenderer`].
                    pub fn render_skill_with<R: ::clifacet::skill::SkillRenderer>(
                        renderer: &R,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let meta = Self::cli_metadata();
                        renderer.render(&meta)
                    }
                }

                #[cfg(all(feature = "skill", feature = "skill-minijinja"))]
                impl #impl_generics ::clifacet::skill::SkillProvider for #name #type_generics #where_clause {
                    type Renderer = ::clifacet::skill::MiniJinjaSkillRenderer;

                    fn skill_renderer() -> Self::Renderer {
                        ::clifacet::skill::MiniJinjaSkillRenderer::new()
                    }

                    fn render_skill() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::render_skill()
                    }

                    fn write_skill(
                        options: &::clifacet::skill::WriteSkillOptions,
                    ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        <#name #type_generics>::write_skill(options)
                    }
                }
            }
        }
    };

    let mcp_inherent_impl = if subcommand_field.is_some() {
        quote! {
            #[cfg(feature = "mcp")]
            impl #impl_generics #name #type_generics #where_clause {
                /// Runs the Model Context Protocol (MCP) server for this application over custom streams.
                ///
                /// # Errors
                ///
                /// Returns a [`Failure`] if running the server fails.
                pub fn run_mcp_server_streams<R: ::std::io::BufRead, W: ::std::io::Write>(
                    reader: R,
                    writer: W,
                ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                    let cmd = <Self as ::clifacet::Facet>::command();
                    let meta = ::clifacet::schema::CliMetadata::from_command(&cmd);
                    let runner_meta = meta.clone();
                    let runner = move |tool_name: &str, arguments: &::serde_json::Value| -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        let argv = ::clifacet::mcp::arguments_to_argv(&runner_meta.name, tool_name, arguments, &runner_meta)?;
                        let app = <::clifacet::App<Self, Self, ::clifacet::app::Plain, <Self as ::clifacet::cli::Cli>::Context>>::for_cli();
                        let mut out = ::std::vec::Vec::new();
                        let mut err = ::std::vec::Vec::new();
                        let exit_code = app.run_with(
                            argv,
                            ::clifacet::policy::Capabilities::headless(),
                            || ::std::boxed::Box::new(::clifacet::prompt::Scripted::default()),
                            &mut out,
                            &mut err,
                        );
                        if exit_code == 0 {
                            let stdout_str = ::std::string::String::from_utf8(out)
                                .map_err(|e| ::clifacet::Failure::execution(e.to_string()))?;
                            let trimmed = stdout_str.trim().to_string();
                            if trimmed.is_empty() {
                                ::core::result::Result::Ok("ok".to_string())
                            } else {
                                ::core::result::Result::Ok(trimmed)
                            }
                        } else {
                            let stderr_str = ::std::string::String::from_utf8(err)
                                .map_err(|e| ::clifacet::Failure::execution(e.to_string()))?;
                            let msg = if stderr_str.trim().is_empty() {
                                let stdout_str = ::std::string::String::from_utf8(out).unwrap_or_default();
                                if stdout_str.trim().is_empty() {
                                    format!("command failed with exit code {exit_code}")
                                } else {
                                    stdout_str.trim().to_string()
                                }
                            } else {
                                stderr_str.trim().to_string()
                            };
                            ::core::result::Result::Err(::clifacet::Failure::execution(msg))
                        }
                    };
                    ::clifacet::mcp::run_server_streams(&meta, runner, reader, writer)
                }

                /// Runs the Model Context Protocol (MCP) stdio tool server for this application.
                ///
                /// # Errors
                ///
                /// Returns a [`Failure`] if running the server fails.
                pub fn run_mcp_server() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                    let stdin = ::std::io::stdin();
                    let stdout = ::std::io::stdout();
                    Self::run_mcp_server_streams(stdin.lock(), stdout.lock())
                }
            }
        }
    } else {
        quote! {
            #[cfg(feature = "mcp")]
            impl #impl_generics #name #type_generics #where_clause {
                /// Runs the Model Context Protocol (MCP) server for this application over custom streams.
                ///
                /// # Errors
                ///
                /// Returns a [`Failure`] if running the server fails.
                pub fn run_mcp_server_streams<R: ::std::io::BufRead, W: ::std::io::Write>(
                    reader: R,
                    writer: W,
                ) -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                    let cmd = <Self as ::clifacet::Facet>::command();
                    let meta = ::clifacet::schema::CliMetadata::from_command(&cmd);
                    let runner = |_tool_name: &str, _arguments: &::serde_json::Value| -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                        ::core::result::Result::Err(::clifacet::Failure::execution("no commands declared"))
                    };
                    ::clifacet::mcp::run_server_streams(&meta, runner, reader, writer)
                }

                /// Runs the Model Context Protocol (MCP) stdio tool server for this application.
                ///
                /// # Errors
                ///
                /// Returns a [`Failure`] if running the server fails.
                pub fn run_mcp_server() -> ::core::result::Result<::std::string::String, ::clifacet::Failure> {
                    let stdin = ::std::io::stdin();
                    let stdout = ::std::io::stdout();
                    Self::run_mcp_server_streams(stdin.lock(), stdout.lock())
                }
            }
        }
    };

    let completions_inherent_impl = quote! {
        #[cfg(feature = "completions")]
        impl #impl_generics #name #type_generics #where_clause {
            /// Generates the shell completion script for this application for `shell`.
            #[must_use]
            pub fn completion_script(shell: ::clifacet::completions::Shell) -> ::std::string::String {
                let cmd = <Self as ::clifacet::Facet>::command();
                ::clifacet::completions::generate_completion_script(shell, &cmd)
            }
        }
    };

    parsed.attrs.insert(
        0,
        syn::parse_quote!(#[derive(::clap::Parser, ::std::clone::Clone)]),
    );
    if !command_attr.is_empty() {
        parsed.attrs.insert(1, syn::parse_quote!(#command_attr));
    }

    Ok(quote! {
        #parsed
        #init_from_parent
        #cli_impl
        #facet_impl
        #collect_impl
        #json_inherent_impl
        #schema_inherent_impl
        #skill_inherent_impl
        #mcp_inherent_impl
        #completions_inherent_impl
    })
}

fn process_clifacet_attr(attr: &Attribute) -> syn::Result<(bool, Option<Path>, bool, Vec<Meta>)> {
    let entries = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
    let mut has_context = false;
    let mut custom_factory: Option<Path> = None;
    let mut has_subcmd = false;
    let mut other_metas = Vec::new();

    for entry in entries {
        if entry.path().is_ident("context") {
            has_context = true;
            match &entry {
                Meta::Path(_) => {}
                Meta::NameValue(nv) => {
                    if let Expr::Path(expr_path) = &nv.value {
                        custom_factory = Some(expr_path.path.clone());
                    } else {
                        return Err(syn::Error::new_spanned(
                            &nv.value,
                            "expected path to context factory function",
                        ));
                    }
                }
                Meta::List(list) => {
                    return Err(syn::Error::new_spanned(
                        list,
                        "invalid `context` attribute: use `#[clifacet(context)]` or `#[clifacet(context = path)]`",
                    ));
                }
            }
        } else if entry.path().is_ident("subcommand") || entry.path().is_ident("subcommands") {
            has_subcmd = true;
        } else {
            other_metas.push(entry);
        }
    }

    Ok((has_context, custom_factory, has_subcmd, other_metas))
}

fn parse_app_args(attribute: TokenStream) -> syn::Result<AppArgs> {
    if attribute.is_empty() {
        return Ok(AppArgs::default());
    }

    let entries =
        syn::parse::Parser::parse2(Punctuated::<Meta, Token![,]>::parse_terminated, attribute)?;

    let mut args = AppArgs::default();
    for entry in entries {
        let Some(ident) = entry.path().get_ident() else {
            return Err(syn::Error::new_spanned(
                &entry,
                "#[clifacet::app] attributes must be plain identifiers",
            ));
        };
        let key = ident.to_string();
        match key.as_str() {
            "name" => {
                if args.name.is_some() {
                    return Err(syn::Error::new_spanned(ident, "`name` is declared twice"));
                }
                args.name = Some(parse_lit_str(&entry)?);
            }
            "version" => {
                if args.version.is_some() {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "`version` is declared twice",
                    ));
                }
                match entry {
                    Meta::Path(_) => args.version = Some(VersionArg::Default),
                    Meta::NameValue(_) => {
                        args.version = Some(VersionArg::Specified(parse_lit_str(&entry)?));
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            entry,
                            "`version` must be a flag or `version = \"...\"`",
                        ));
                    }
                }
            }
            "about" => {
                if args.about.is_some() {
                    return Err(syn::Error::new_spanned(ident, "`about` is declared twice"));
                }
                args.about = Some(parse_lit_str(&entry)?);
            }
            "long_about" => {
                if args.long_about.is_some() {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "`long_about` is declared twice",
                    ));
                }
                args.long_about = Some(parse_lit_str(&entry)?);
            }
            "author" => {
                if args.author.is_some() {
                    return Err(syn::Error::new_spanned(ident, "`author` is declared twice"));
                }
                args.author = Some(parse_lit_str(&entry)?);
            }
            "before_run" => {
                if args.before_run.is_some() {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "`before_run` is declared twice",
                    ));
                }
                args.before_run = Some(parse_path(&entry)?);
            }
            "after_success" => {
                if args.after_success.is_some() {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "`after_success` is declared twice",
                    ));
                }
                args.after_success = Some(parse_path(&entry)?);
            }
            "after_failure" => {
                if args.after_failure.is_some() {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "`after_failure` is declared twice",
                    ));
                }
                args.after_failure = Some(parse_path(&entry)?);
            }
            "input" => {
                if args.input.is_some() {
                    return Err(syn::Error::new_spanned(ident, "`input` is declared twice"));
                }
                args.input = Some(parse_type(&entry)?);
            }
            "commands" => match entry {
                Meta::Path(_) => args.commands = Some(None),
                Meta::NameValue(nv) => {
                    if let syn::Expr::Path(ep) = nv.value {
                        args.commands = Some(ep.path.get_ident().cloned());
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        entry,
                        "`commands` must be a flag or `commands = Commands`",
                    ));
                }
            },
            "schema" => match entry {
                Meta::Path(_) => args.schema = Some(true),
                Meta::NameValue(nv) => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Bool(b),
                        ..
                    }) = nv.value
                    {
                        args.schema = Some(b.value());
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv,
                            "`schema` must be a flag or `schema = bool`",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        entry,
                        "`schema` must be a flag or `schema = bool`",
                    ));
                }
            },
            "skill" => match entry {
                Meta::Path(_) => args.skill = Some(SkillConfig::Default),
                Meta::NameValue(nv) => {
                    if let syn::Expr::Path(ep) = nv.value {
                        args.skill = Some(SkillConfig::Renderer(ep.path));
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv,
                            "`skill` must be a flag, `skill = RendererPath`, or `skill(...)`",
                        ));
                    }
                }
                Meta::List(ml) => {
                    let mut config = None;
                    ml.parse_nested_meta(|meta| {
                        let sub_ident = meta
                            .path
                            .get_ident()
                            .ok_or_else(|| meta.error("expected identifier"))?;
                        let sub_key = sub_ident.to_string();
                        match sub_key.as_str() {
                            "template" => {
                                let lit: syn::LitStr = meta.value()?.parse()?;
                                config = Some(SkillConfig::Template(lit));
                                Ok(())
                            }
                            "template_path" => {
                                let lit: syn::LitStr = meta.value()?.parse()?;
                                config = Some(SkillConfig::TemplatePath(lit));
                                Ok(())
                            }
                            "renderer" => {
                                let path: syn::Path = meta.value()?.parse()?;
                                config = Some(SkillConfig::Renderer(path));
                                Ok(())
                            }
                            _ => Err(meta.error(format!(
                                "unknown key `{sub_key}` in skill: supported keys are `template`, `template_path`, `renderer`"
                            ))),
                        }
                    })?;
                    args.skill = Some(config.unwrap_or(SkillConfig::Default));
                }
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    ident,
                    format!(
                        "unknown key `{key}` in #[clifacet::app]: supported keys are `name`, \
                         `version`, `about`, `long_about`, `author`, `before_run`, \
                         `after_success`, `after_failure`, `input`, `commands`, `schema`, `skill`"
                    ),
                ));
            }
        }
    }
    Ok(args)
}

fn parse_lit_str(meta: &Meta) -> syn::Result<LitStr> {
    match meta {
        Meta::NameValue(nv) => {
            if let Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(lit_str) = &expr_lit.lit {
                    return Ok(lit_str.clone());
                }
            }
            Err(syn::Error::new_spanned(
                &nv.value,
                "expected a string literal",
            ))
        }
        _ => Err(syn::Error::new_spanned(meta, "expected key = \"value\"")),
    }
}

fn parse_path(meta: &Meta) -> syn::Result<Path> {
    match meta {
        Meta::NameValue(nv) => {
            if let Expr::Path(expr_path) = &nv.value {
                return Ok(expr_path.path.clone());
            }
            Err(syn::Error::new_spanned(&nv.value, "expected a path"))
        }
        _ => Err(syn::Error::new_spanned(meta, "expected key = path")),
    }
}

fn parse_type(meta: &Meta) -> syn::Result<Type> {
    match meta {
        Meta::NameValue(nv) => {
            let expr = &nv.value;
            syn::parse2::<Type>(quote!(#expr))
        }
        _ => Err(syn::Error::new_spanned(meta, "expected key = Type")),
    }
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(_) = &segment.arguments {
                    return true;
                }
            }
        }
    }
    false
}
