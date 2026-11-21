//! `#[clifacet::commands]`: maps struct methods in an `impl` block to CLI commands (decision-24).

use heck::{ToKebabCase as _, ToUpperCamelCase as _};
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Attribute, FnArg, Ident, ItemImpl, Meta, Pat, Token, Type};

#[allow(clippy::similar_names)]
pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut item_impl = syn::parse2::<ItemImpl>(item)?;
    let struct_ident = match &*item_impl.self_ty {
        Type::Path(tp) => {
            let seg =
                tp.path.segments.last().ok_or_else(|| {
                    syn::Error::new_spanned(tp, "expected struct path in impl block")
                })?;
            seg.ident.clone()
        }
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "#[clifacet::commands] can only be applied to an impl block for a struct",
            ));
        }
    };

    let (custom_enum, custom_field, is_group) = parse_commands_args(attribute)?;
    let enum_ident =
        custom_enum.unwrap_or_else(|| quote::format_ident!("{}Commands", struct_ident));
    let subcmd_field = custom_field.unwrap_or_else(|| quote::format_ident!("command"));
    let (impl_generics, type_generics, where_clause) = item_impl.generics.split_for_impl();

    let mut variants = Vec::new();
    let mut dispatch_arms = Vec::new();
    let mut dispatch_arms_async = Vec::new();
    let mut complete_providers = Vec::new();

    for impl_item in &mut item_impl.items {
        let syn::ImplItem::Fn(method) = impl_item else {
            continue;
        };

        let fn_name = method.sig.ident.to_string();
        if fn_name == "before_run"
            || fn_name == "after_success"
            || fn_name == "after_failure"
            || fn_name == "init_context"
        {
            continue;
        }

        let mut skip = false;
        method.attrs.retain(|attr| {
            if attr.path().is_ident(crate::field::ATTRIBUTE) {
                if let Ok(meta) = attr.parse_args::<syn::Ident>() {
                    if meta == "skip" {
                        skip = true;
                        return false;
                    }
                }
            }
            true
        });
        if skip {
            continue;
        }

        let mut has_receiver = false;
        let mut params = Vec::new();

        for arg in &mut method.sig.inputs {
            match arg {
                FnArg::Receiver(_) => {
                    has_receiver = true;
                }
                FnArg::Typed(pat_type) => {
                    params.push(pat_type.clone());
                    // Strip #[clifacet(...)] from method parameter so rustc doesn't warn
                    pat_type
                        .attrs
                        .retain(|a| !a.path().is_ident(crate::field::ATTRIBUTE));
                }
            }
        }

        if !has_receiver {
            return Err(syn::Error::new_spanned(
                &method.sig,
                "command method in #[clifacet::commands] must have `&self` receiver",
            ));
        }

        let doc_attrs: Vec<Attribute> = method
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("doc"))
            .cloned()
            .collect();

        let is_async = method.sig.asyncness.is_some();
        let method_ident = &method.sig.ident;
        let variant_ident = quote::format_ident!("{}", fn_name.to_upper_camel_case());
        let cmd_name = fn_name.to_kebab_case();

        let mut variant_fields = Vec::new();
        let mut call_args_sync = Vec::new();
        let mut call_args_async = Vec::new();
        let mut field_names = Vec::new();
        let mut subcommand_tuple_param = None;

        for param in params {
            let pat_ident = match &*param.pat {
                Pat::Ident(pi) => pi.ident.clone(),
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "expected identifier pattern in command parameter",
                    ));
                }
            };
            let ty = &param.ty;

            if is_run_cx_type(ty) {
                call_args_sync.push(quote!(&::clifacet::r#async::RunCx::empty()));
                call_args_async.push(quote!(_run));
                continue;
            }

            let mut field_attrs = Vec::new();
            let mut is_subcmd = false;
            let mut complete_fn = None;

            for attr in &param.attrs {
                if attr.path().is_ident("subcommand") {
                    is_subcmd = true;
                } else if attr.path().is_ident(crate::field::ATTRIBUTE) {
                    let entries =
                        attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
                    let mut other_metas = Vec::new();
                    for entry in entries {
                        if entry.path().is_ident("subcommand")
                            || entry.path().is_ident("subcommands")
                        {
                            is_subcmd = true;
                        } else if entry.path().is_ident("complete")
                            || entry.path().is_ident("choices")
                        {
                            if let Meta::NameValue(nv) = entry {
                                if let syn::Expr::Path(ep) = nv.value {
                                    complete_fn = Some(ep.path);
                                }
                            }
                        } else {
                            other_metas.push(entry);
                        }
                    }
                    if !other_metas.is_empty() {
                        let sub_attr: Attribute = syn::parse_quote!(#[clifacet(#(#other_metas),*)]);
                        let declared = crate::field::parse(&sub_attr, ty)?;
                        field_attrs.extend(declared.attributes());
                    }
                } else if attr.path().is_ident("arg") {
                    field_attrs.push(attr.clone());
                }
            }

            if let Some(cb) = complete_fn {
                complete_providers.push((cmd_name.clone(), pat_ident.to_string(), cb));
            }

            if is_subcmd {
                subcommand_tuple_param = Some((pat_ident.clone(), ty.clone()));
            }

            variant_fields.push(quote! {
                #(#field_attrs)*
                #pat_ident: #ty
            });
            call_args_sync.push(quote! {
                ::std::clone::Clone::clone(#pat_ident)
            });
            call_args_async.push(quote! {
                ::std::clone::Clone::clone(#pat_ident)
            });
            field_names.push(pat_ident);
        }

        if let Some((sub_ident, sub_ty)) = subcommand_tuple_param {
            variants.push(quote! {
                #[command(name = #cmd_name)]
                #(#doc_attrs)*
                #variant_ident(#sub_ty),
            });
            let call_outcome_sync = if is_async {
                quote! {
                    ::clifacet::cli::block_on(
                        self.#method_ident(::std::clone::Clone::clone(#sub_ident))
                    )
                }
            } else {
                quote!(self.#method_ident(::std::clone::Clone::clone(#sub_ident)))
            };
            let call_outcome_async = if is_async {
                quote! {
                    self.#method_ident(::std::clone::Clone::clone(#sub_ident)).await
                }
            } else {
                quote!(self.#method_ident(::std::clone::Clone::clone(#sub_ident)))
            };
            dispatch_arms.push(quote! {
                ::core::option::Option::Some(#enum_ident::#variant_ident(#sub_ident)) => {
                    let outcome = #call_outcome_sync;
                    let res = ::clifacet::IntoOutcome::into_outcome(outcome)?;
                    let is_dry_run = {
                        use ::clifacet::cli::check_dry_run::FallbackIsDryRun as _;
                        self.__clifacet_is_dry_run()
                    };
                    let is_json = {
                        use ::clifacet::cli::check_json::FallbackIsJson as _;
                        self.__clifacet_is_json()
                    };
                    {
                        use ::clifacet::cli::auto_wire_json::FallbackFormatOutcome as _;
                        res.format_outcome(is_dry_run, is_json)
                    }
                }
            });
            dispatch_arms_async.push(quote! {
                ::core::option::Option::Some(#enum_ident::#variant_ident(#sub_ident)) => {
                    let outcome = #call_outcome_async;
                    let res = ::clifacet::IntoOutcome::into_outcome(outcome)?;
                    let is_dry_run = {
                        use ::clifacet::cli::check_dry_run::FallbackIsDryRun as _;
                        self.__clifacet_is_dry_run()
                    };
                    let is_json = {
                        use ::clifacet::cli::check_json::FallbackIsJson as _;
                        self.__clifacet_is_json()
                    };
                    {
                        use ::clifacet::cli::auto_wire_json_async::FallbackFormatOutcomeAsync as _;
                        res.format_outcome_async(is_dry_run, is_json).await
                    }
                }
            });
        } else if variant_fields.is_empty() {
            variants.push(quote! {
                #[command(name = #cmd_name)]
                #(#doc_attrs)*
                #variant_ident,
            });
            let call_outcome_sync = if is_async {
                quote!(::clifacet::cli::block_on(self.#method_ident(#(#call_args_sync),*)))
            } else {
                quote!(self.#method_ident(#(#call_args_sync),*))
            };
            let call_outcome_async = if is_async {
                quote!(self.#method_ident(#(#call_args_async),*).await)
            } else {
                quote!(self.#method_ident(#(#call_args_async),*))
            };
            dispatch_arms.push(quote! {
                ::core::option::Option::Some(#enum_ident::#variant_ident) => {
                    let outcome = #call_outcome_sync;
                    let res = ::clifacet::IntoOutcome::into_outcome(outcome)?;
                    let is_dry_run = {
                        use ::clifacet::cli::check_dry_run::FallbackIsDryRun as _;
                        self.__clifacet_is_dry_run()
                    };
                    let is_json = {
                        use ::clifacet::cli::check_json::FallbackIsJson as _;
                        self.__clifacet_is_json()
                    };
                    {
                        use ::clifacet::cli::auto_wire_json::FallbackFormatOutcome as _;
                        res.format_outcome(is_dry_run, is_json)
                    }
                }
            });
            dispatch_arms_async.push(quote! {
                ::core::option::Option::Some(#enum_ident::#variant_ident) => {
                    let outcome = #call_outcome_async;
                    let res = ::clifacet::IntoOutcome::into_outcome(outcome)?;
                    let is_dry_run = {
                        use ::clifacet::cli::check_dry_run::FallbackIsDryRun as _;
                        self.__clifacet_is_dry_run()
                    };
                    let is_json = {
                        use ::clifacet::cli::check_json::FallbackIsJson as _;
                        self.__clifacet_is_json()
                    };
                    {
                        use ::clifacet::cli::auto_wire_json_async::FallbackFormatOutcomeAsync as _;
                        res.format_outcome_async(is_dry_run, is_json).await
                    }
                }
            });
        } else {
            variants.push(quote! {
                #[command(name = #cmd_name)]
                #(#doc_attrs)*
                #variant_ident {
                    #(#variant_fields),*
                },
            });
            let call_outcome_sync = if is_async {
                quote! {
                    ::clifacet::cli::block_on(self.#method_ident(#(#call_args_sync),*))
                }
            } else {
                quote!(self.#method_ident(#(#call_args_sync),*))
            };
            let call_outcome_async = if is_async {
                quote! {
                    self.#method_ident(#(#call_args_async),*).await
                }
            } else {
                quote!(self.#method_ident(#(#call_args_async),*))
            };
            dispatch_arms.push(quote! {
                ::core::option::Option::Some(#enum_ident::#variant_ident { #(#field_names),* }) => {
                    let outcome = #call_outcome_sync;
                    let res = ::clifacet::IntoOutcome::into_outcome(outcome)?;
                    let is_dry_run = {
                        use ::clifacet::cli::check_dry_run::FallbackIsDryRun as _;
                        self.__clifacet_is_dry_run()
                    };
                    let is_json = {
                        use ::clifacet::cli::check_json::FallbackIsJson as _;
                        self.__clifacet_is_json()
                    };
                    {
                        use ::clifacet::cli::auto_wire_json::FallbackFormatOutcome as _;
                        res.format_outcome(is_dry_run, is_json)
                    }
                }
            });
            dispatch_arms_async.push(quote! {
                ::core::option::Option::Some(#enum_ident::#variant_ident { #(#field_names),* }) => {
                    let outcome = #call_outcome_async;
                    let res = ::clifacet::IntoOutcome::into_outcome(outcome)?;
                    let is_dry_run = {
                        use ::clifacet::cli::check_dry_run::FallbackIsDryRun as _;
                        self.__clifacet_is_dry_run()
                    };
                    let is_json = {
                        use ::clifacet::cli::check_json::FallbackIsJson as _;
                        self.__clifacet_is_json()
                    };
                    {
                        use ::clifacet::cli::auto_wire_json_async::FallbackFormatOutcomeAsync as _;
                        res.format_outcome_async(is_dry_run, is_json).await
                    }
                }
            });
        }
    }

    let has_user_schema = item_impl.items.iter().any(|item| {
        if let syn::ImplItem::Fn(m) = item {
            m.sig.ident == "schema"
        } else {
            false
        }
    });

    if !is_group && !has_user_schema {
        variants.push(quote! {
            #[cfg(feature = "schema")]
            #[command(name = "schema", about = "Print the machine-readable CLI JSON schema")]
            Schema,
        });
        let schema_arm = quote! {
            #[cfg(feature = "schema")]
            ::core::option::Option::Some(#enum_ident::Schema) => {
                let cmd = <Self as ::clifacet::Facet>::command();
                let meta = ::clifacet::schema::CliMetadata::from_command(&cmd);
                let json = ::clifacet::schema::to_json_schema_pretty(&meta)
                    .map_err(|e| ::clifacet::Failure::execution(format!("cannot generate schema: {e}")))?;
                ::core::result::Result::Ok(json)
            }
        };
        dispatch_arms.push(schema_arm.clone());
        dispatch_arms_async.push(schema_arm);
    }

    let has_user_write_skill = item_impl.items.iter().any(|item| {
        if let syn::ImplItem::Fn(m) = item {
            m.sig.ident == "write_skill"
        } else {
            false
        }
    });

    if !is_group && !has_user_write_skill {
        variants.push(quote! {
            #[cfg(feature = "skill")]
            #[command(name = "write-skill", about = "Generate or verify agent skill documentation")]
            WriteSkill {
                /// Write to standard output instead of a file
                #[arg(long)]
                stdout: bool,

                /// Explicit output file path
                #[arg(short, long)]
                output: Option<::std::path::PathBuf>,

                /// Check on-disk file against current rendering and exit with non-zero status on drift
                #[arg(long)]
                check: bool,
            },
        });
        let write_skill_arm = quote! {
            #[cfg(feature = "skill")]
            ::core::option::Option::Some(#enum_ident::WriteSkill { stdout, output, check }) => {
                let options = ::clifacet::skill::WriteSkillOptions {
                    stdout: *stdout,
                    output: output.clone(),
                    check: *check,
                };
                let res = Self::write_skill(&options)?;
                if *stdout {
                    ::core::result::Result::Ok(res.trim_end_matches('\n').to_string())
                } else {
                    ::core::result::Result::Ok(res)
                }
            }
        };
        dispatch_arms.push(write_skill_arm.clone());
        dispatch_arms_async.push(write_skill_arm);
    }

    let has_user_mcp = item_impl.items.iter().any(|item| {
        if let syn::ImplItem::Fn(m) = item {
            m.sig.ident == "mcp"
        } else {
            false
        }
    });

    if !is_group && !has_user_mcp {
        variants.push(quote! {
            #[cfg(feature = "mcp")]
            #[command(name = "mcp", about = "Run Model Context Protocol (MCP) stdio tool server")]
            Mcp,
        });
        let mcp_arm = quote! {
            #[cfg(feature = "mcp")]
            ::core::option::Option::Some(#enum_ident::Mcp) => {
                Self::run_mcp_server()
            }
        };
        dispatch_arms.push(mcp_arm.clone());
        dispatch_arms_async.push(mcp_arm);
    }

    let has_user_completions = item_impl.items.iter().any(|item| {
        if let syn::ImplItem::Fn(m) = item {
            m.sig.ident == "completions"
        } else {
            false
        }
    });

    if !is_group && !has_user_completions {
        variants.push(quote! {
            #[cfg(feature = "completions")]
            #[command(name = "completions", about = "Generate shell completion scripts")]
            Completions {
                /// Internal dynamic completion evaluation flag
                #[arg(long, hide = true)]
                complete: bool,

                /// Target shell: bash, zsh, fish, or PowerShell
                #[arg(value_name = "SHELL", trailing_var_arg = true, allow_hyphen_values = true)]
                args: Vec<String>,
            },
        });
        let completions_arm = quote! {
            #[cfg(feature = "completions")]
            ::core::option::Option::Some(#enum_ident::Completions { complete, args }) => {
                if *complete {
                    let cx = <Self as ::clifacet::Cli>::init_context(self)?;
                    let cmd = <Self as ::clifacet::Facet>::command();
                    let meta = ::clifacet::schema::CliMetadata::from_command(&cmd);
                    let candidates = ::clifacet::completions::resolve_completions(
                        &meta,
                        args,
                        |subcmd, arg, current| {
                            Self::complete_argument(&cx, subcmd, arg, current)
                        },
                    );
                    ::core::result::Result::Ok(candidates.join("\n"))
                } else if let ::core::option::Option::Some(first) = args.first() {
                    let target_shell: ::clifacet::completions::Shell = first.parse()?;
                    let cmd = <Self as ::clifacet::Facet>::command();
                    let script = ::clifacet::completions::generate_completion_script(target_shell, &cmd);
                    ::core::result::Result::Ok(script)
                } else {
                    ::core::result::Result::Err(::clifacet::Failure::validation("missing shell argument: specify bash, zsh, fish, or PowerShell"))
                }
            }
        };
        dispatch_arms.push(completions_arm.clone());
        dispatch_arms_async.push(completions_arm);
    }

    let enum_def = quote! {
        #[derive(::clifacet::clap::Subcommand, ::std::clone::Clone, ::std::fmt::Debug)]
        pub enum #enum_ident {
            #(#variants)*
        }

        impl ::clifacet::facet::__derive::Tree for #enum_ident {
            type Input = Self;
            fn into_input(self) -> ::core::result::Result<Self::Input, ::clifacet::Failure> {
                ::core::result::Result::Ok(self)
            }
        }

        impl ::clifacet::prompt::Collect for #enum_ident {
            fn collect(
                self,
                _session: &mut ::clifacet::prompt::Session,
            ) -> ::core::result::Result<Self, ::clifacet::Failure> {
                ::core::result::Result::Ok(self)
            }
        }
    };

    let handler_impl = if is_group {
        quote! {
            impl #impl_generics #struct_ident #type_generics #where_clause {
                /// Executes the chosen subcommand on this controller or group.
                pub fn execute(&self) -> ::core::result::Result<String, ::clifacet::Failure> {
                    match &self.#subcmd_field {
                        #(#dispatch_arms)*
                        ::core::option::Option::None => {
                            ::core::result::Result::Ok(String::new())
                        }
                    }
                }

                #[cfg(feature = "async")]
                /// Executes the chosen subcommand on this controller or group asynchronously.
                #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
                pub async fn execute_async(
                    &self,
                    _run: &::clifacet::r#async::RunCx,
                ) -> ::core::result::Result<String, ::clifacet::Failure> {
                    match &self.#subcmd_field {
                        #(#dispatch_arms_async)*
                        ::core::option::Option::None => {
                            ::core::result::Result::Ok(String::new())
                        }
                    }
                }
            }
        }
    } else {
        quote! {
            impl #impl_generics #struct_ident #type_generics #where_clause {
                /// Executes the chosen subcommand on this controller or group.
                pub fn execute(&self) -> ::core::result::Result<String, ::clifacet::Failure> {
                    match &self.#subcmd_field {
                        #(#dispatch_arms)*
                        ::core::option::Option::None => {
                            ::core::result::Result::Ok(String::new())
                        }
                    }
                }

                #[cfg(feature = "async")]
                /// Executes the chosen subcommand on this controller or group asynchronously.
                #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
                pub async fn execute_async(
                    &self,
                    _run: &::clifacet::r#async::RunCx,
                ) -> ::core::result::Result<String, ::clifacet::Failure> {
                    match &self.#subcmd_field {
                        #(#dispatch_arms_async)*
                        ::core::option::Option::None => {
                            ::core::result::Result::Ok(String::new())
                        }
                    }
                }
            }

            impl #impl_generics ::clifacet::Handler<<#struct_ident as ::clifacet::Cli>::Context> for #struct_ident #type_generics #where_clause {
                type Input = Self;
                type Output = String;

                fn handle(
                    &self,
                    _input: Self::Input,
                    _cx: &<#struct_ident as ::clifacet::Cli>::Context,
                ) -> ::core::result::Result<Self::Output, ::clifacet::Failure> {
                    self.execute()
                }
            }

            #[cfg(feature = "async")]
            #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
            impl #impl_generics ::clifacet::r#async::LocalAsyncHandler<<#struct_ident as ::clifacet::Cli>::Context> for #struct_ident #type_generics #where_clause {
                type Input = Self;
                type Output = String;

                #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
                fn handle_local(
                    &self,
                    _input: Self::Input,
                    _cx: &<#struct_ident as ::clifacet::Cli>::Context,
                    run: &::clifacet::r#async::RunCx,
                ) -> impl ::core::future::Future<Output = ::core::result::Result<Self::Output, ::clifacet::Failure>> {
                    self.execute_async(run)
                }
            }
        }
    };

    let mut complete_arms = Vec::new();
    for (c_name, a_name, cb) in &complete_providers {
        let a_kebab = a_name.to_kebab_case();
        let flag = format!("--{a_kebab}");
        complete_arms.push(quote! {
            (#c_name, #a_name) | (#c_name, #a_kebab) | (#c_name, #flag) => {
                let items = {
                    use ::clifacet::cli::auto_wire_choice::{
                        CallSyncChoiceResult as _, ChoiceWrap,
                    };
                    #[cfg(feature = "async")]
                    use ::clifacet::cli::auto_wire_choice::CallAsyncChoiceResult as _;

                    ChoiceWrap(#cb(cx, current)).call_choice_result()
                };
                ::core::option::Option::Some(items)
            }
        });
    }

    let complete_impl = if is_group {
        quote! {}
    } else {
        quote! {
            #[cfg(feature = "completions")]
            impl #impl_generics #struct_ident #type_generics #where_clause {
                /// Dispatches dynamic argument completion queries against this controller's context.
                pub fn complete_argument(
                    cx: &<#struct_ident as ::clifacet::Cli>::Context,
                    cmd: &str,
                    arg: &str,
                    current: &str,
                ) -> ::core::option::Option<::std::vec::Vec<::std::string::String>> {
                    match (cmd, arg) {
                        #(#complete_arms)*
                        _ => ::core::option::Option::None,
                    }
                }
            }
        }
    };

    Ok(quote! {
        #item_impl
        #enum_def
        #handler_impl
        #complete_impl
    })
}

fn parse_commands_args(
    attribute: TokenStream,
) -> syn::Result<(Option<Ident>, Option<Ident>, bool)> {
    if attribute.is_empty() {
        return Ok((None, None, false));
    }

    if let Ok(ident) = syn::parse2::<Ident>(attribute.clone()) {
        if ident == "group" {
            return Ok((None, None, true));
        }
        return Ok((Some(ident), None, false));
    }

    let entries =
        syn::parse::Parser::parse2(Punctuated::<Meta, Token![,]>::parse_terminated, attribute)?;

    let mut custom_enum = None;
    let mut custom_field = None;
    let mut is_group = false;

    for entry in entries {
        match entry {
            Meta::Path(p) => {
                if let Some(ident) = p.get_ident() {
                    if ident == "group" {
                        is_group = true;
                    } else {
                        custom_enum = Some(ident.clone());
                    }
                }
            }
            Meta::NameValue(nv) => {
                if nv.path.is_ident("enum") || nv.path.is_ident("name") {
                    if let syn::Expr::Path(ep) = nv.value {
                        custom_enum = ep.path.get_ident().cloned();
                    }
                } else if nv.path.is_ident("field") {
                    if let syn::Expr::Path(ep) = nv.value {
                        custom_field = ep.path.get_ident().cloned();
                    }
                }
            }
            _ => {}
        }
    }

    Ok((custom_enum, custom_field, is_group))
}

fn is_run_cx_type(ty: &Type) -> bool {
    let check_path = |p: &syn::Path| p.segments.last().is_some_and(|seg| seg.ident == "RunCx");
    match ty {
        Type::Reference(tr) => match &*tr.elem {
            Type::Path(tp) => check_path(&tp.path),
            _ => false,
        },
        Type::Path(tp) => check_path(&tp.path),
        _ => false,
    }
}
