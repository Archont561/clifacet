//! `#[clifacet::planned]`: the `Handler` a `PlannedWith` impl implies (decision-19 item 2).
//!
//! `Planned` gets `Command` from a blanket impl in the library. Its twin with a context,
//! `PlannedWith<Cx>`, cannot get `Handler<Cx>` the same way: that blanket would overlap the
//! one every `Command` already has (E0119). So the bridge is written here, beside the impl
//! it comes from, and it is the same bridge the blanket would have been — `handle` executes
//! the plan and `preview` returns it — so a dry run and a real run still share one path.

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
use syn::{GenericArgument, ItemImpl, PathArguments, Type};

/// The attribute's own name, for messages.
const ATTRIBUTE: &str = "clifacet::planned";

// The item is returned alongside errors, so the macro keeps it rather than dropping it:
// taking it by reference would copy it out for nothing.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn expand(attribute: &TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attribute.is_empty() {
        return Err(syn::Error::new(
            attribute.span(),
            format!("`#[{ATTRIBUTE}]` takes no arguments: the impl it is on says everything"),
        ));
    }
    let Ok(implementation) = syn::parse2::<ItemImpl>(item.clone()) else {
        return Err(syn::Error::new(item.span(), wrong_item()));
    };
    let (context, is_async) = context_of(&implementation)?;
    let self_type = &implementation.self_ty;
    let (generics, _, where_clause) = implementation.generics.split_for_impl();
    if is_async {
        Ok(quote! {
            #implementation

            #[cfg(feature = "async")]
            impl #generics ::clifacet::r#async::AsyncHandler<#context> for #self_type #where_clause {
                type Input = <Self as ::clifacet::r#async::AsyncPlannedWith<#context>>::Input;
                type Output = <Self as ::clifacet::r#async::AsyncPlannedWith<#context>>::Output;

                fn handle(
                    &self,
                    input: Self::Input,
                    cx: &#context,
                    _run: &::clifacet::r#async::RunCx,
                ) -> impl ::core::future::Future<Output = ::core::result::Result<Self::Output, ::clifacet::Failure>> + ::core::marker::Send {
                    async move {
                        let plan = ::clifacet::r#async::AsyncPlannedWith::<#context>::plan(self, input, cx).await?;
                        plan.execute().await
                    }
                }

                fn preview<'__clifacet>(
                    &'__clifacet self,
                    input: Self::Input,
                    cx: &'__clifacet #context,
                ) -> impl ::core::future::Future<
                    Output = ::core::result::Result<
                        ::clifacet::r#async::AsyncPlan<'__clifacet, Self::Output>,
                        ::clifacet::Failure,
                    >,
                > + ::core::marker::Send + '__clifacet {
                    ::clifacet::r#async::AsyncPlannedWith::<#context>::plan(self, input, cx)
                }
            }

            impl #generics ::clifacet::Handler<#context> for #self_type #where_clause {
                type Input = <Self as ::clifacet::r#async::AsyncPlannedWith<#context>>::Input;
                type Output = <Self as ::clifacet::r#async::AsyncPlannedWith<#context>>::Output;

                fn handle(
                    &self,
                    input: Self::Input,
                    cx: &#context,
                ) -> ::core::result::Result<Self::Output, ::clifacet::Failure> {
                    let run = ::clifacet::r#async::RunCx::empty();
                    ::clifacet::cli::block_on(::clifacet::r#async::AsyncHandler::<#context>::handle(self, input, cx, &run))
                }

                fn preview<'__clifacet>(
                    &'__clifacet self,
                    input: Self::Input,
                    cx: &'__clifacet #context,
                ) -> ::core::result::Result<
                    ::clifacet::command::Plan<'__clifacet, Self::Output>,
                    ::clifacet::Failure,
                > {
                    let async_plan = ::clifacet::cli::block_on(::clifacet::r#async::AsyncPlannedWith::<#context>::plan(self, input, cx))?;
                    let steps = async_plan.steps_slice().to_vec();
                    let mut plan = ::clifacet::command::Plan::new(move || {
                        ::clifacet::cli::block_on(async_plan.execute())
                    });
                    for step in steps {
                        plan = plan.step(step);
                    }
                    ::core::result::Result::Ok(plan)
                }
            }
        })
    } else {
        Ok(quote! {
            #implementation

            impl #generics ::clifacet::Handler<#context> for #self_type #where_clause {
                type Input = <Self as ::clifacet::command::PlannedWith<#context>>::Input;
                type Output = <Self as ::clifacet::command::PlannedWith<#context>>::Output;

                fn handle(
                    &self,
                    input: Self::Input,
                    cx: &#context,
                ) -> ::core::result::Result<Self::Output, ::clifacet::Failure> {
                    ::clifacet::command::PlannedWith::<#context>::plan(self, input, cx)?.execute()
                }

                fn preview<'__clifacet>(
                    &'__clifacet self,
                    input: Self::Input,
                    cx: &'__clifacet #context,
                ) -> ::core::result::Result<
                    ::clifacet::command::Plan<'__clifacet, Self::Output>,
                    ::clifacet::Failure,
                > {
                    ::clifacet::command::PlannedWith::<#context>::plan(self, input, cx)
                }
            }
        })
    }
}

/// The `Cx` of `impl PlannedWith<Cx> for T` or `impl AsyncPlannedWith<Cx> for T`, and whether it is async.
fn context_of(implementation: &ItemImpl) -> syn::Result<(&Type, bool)> {
    let Some((negated, path, _)) = &implementation.trait_ else {
        return Err(syn::Error::new(implementation.self_ty.span(), wrong_item()));
    };
    let last = path.segments.last();
    let is_async = match last.map(|segment| segment.ident.to_string()).as_deref() {
        Some("PlannedWith") => false,
        Some("AsyncPlannedWith") => true,
        _ => return Err(syn::Error::new(path.span(), wrong_item())),
    };
    if negated.is_some() {
        return Err(syn::Error::new(path.span(), wrong_item()));
    }
    let arguments = last.map(|segment| &segment.arguments);
    if let Some(PathArguments::AngleBracketed(arguments)) = arguments
        && let [GenericArgument::Type(context)] =
            arguments.args.iter().collect::<Vec<_>>().as_slice()
    {
        return Ok((context, is_async));
    }
    Err(syn::Error::new(
        path.span(),
        if is_async {
            "`AsyncPlannedWith` names the context it plans with: write `impl AsyncPlannedWith<Store> for …`"
        } else {
            "`PlannedWith` names the context it plans with: write `impl PlannedWith<Store> for …`"
        },
    ))
}

fn wrong_item() -> String {
    format!(
        "`#[{ATTRIBUTE}]` goes on an `impl PlannedWith<Context> for Command` or `impl AsyncPlannedWith<Context> for Command` block, and adds \
         the `Handler<Context>` that executes its plan; a planned command without a context \
         implements `Planned` (or `AsyncPlanned`) and needs no attribute"
    )
}
