//! `#[clifacet::group(...)]`: declarative nested subcommand group controller macro (decision-24).

use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Data, DeriveInput, Fields, Meta, Token};

pub(crate) fn expand(attribute: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut parsed = syn::parse2::<DeriveInput>(item)?;
    let Data::Struct(data_struct) = &mut parsed.data else {
        return Err(syn::Error::new_spanned(
            &parsed,
            "#[clifacet::group] can only be applied to a struct with named fields",
        ));
    };
    let Fields::Named(fields_named) = &mut data_struct.fields else {
        return Err(syn::Error::new_spanned(
            &parsed,
            "#[clifacet::group] can only be applied to a struct with named fields",
        ));
    };

    let has_commands_flag = parse_group_args(attribute)?;

    let mut extract_fields = Vec::new();
    let mut subcommand_field = None;

    for field in &mut fields_named.named {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(&field, "expected named field"))?;

        let mut is_extract = false;
        let mut is_subcommand = false;
        let mut remaining_attrs = Vec::new();

        for attr in field.attrs.drain(..) {
            if attr.path().is_ident("extract") || attr.path().is_ident("context") {
                is_extract = true;
            } else if attr.path().is_ident("subcommand") {
                is_subcommand = true;
            } else if attr.path().is_ident(crate::field::ATTRIBUTE) {
                let entries =
                    attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
                let mut other_metas = Vec::new();
                for entry in entries {
                    if entry.path().is_ident("extract") || entry.path().is_ident("context") {
                        is_extract = true;
                    } else if entry.path().is_ident("subcommand")
                        || entry.path().is_ident("subcommands")
                    {
                        is_subcommand = true;
                    } else {
                        other_metas.push(entry);
                    }
                }
                if !other_metas.is_empty() {
                    remaining_attrs.push(syn::parse_quote!(#[clifacet(#(#other_metas),*)]));
                }
            } else {
                remaining_attrs.push(attr);
            }
        }

        if is_extract {
            extract_fields.push((field_ident.clone(), field.ty.clone()));
            let orig_ty = &field.ty;
            field.ty = syn::parse_quote!(::clifacet::ContextCell<#orig_ty>);
            remaining_attrs.push(syn::parse_quote!(#[arg(skip)]));
        }

        if is_subcommand {
            subcommand_field = Some(field_ident);
            remaining_attrs.push(syn::parse_quote!(#[command(subcommand)]));
        }

        field.attrs = remaining_attrs;
    }

    if has_commands_flag && subcommand_field.is_none() {
        let enum_ident = quote::format_ident!("{}Commands", parsed.ident);
        let cmd_ident = quote::format_ident!("command");
        subcommand_field = Some(cmd_ident.clone());
        fields_named.named.push(syn::parse_quote! {
            #[command(subcommand)]
            pub #cmd_ident: Option<#enum_ident>
        });
    }

    crate::command::forward_field_attributes(&mut parsed)?;

    let name = &parsed.ident;
    let (impl_generics, type_generics, where_clause) = parsed.generics.split_for_impl();

    let extract_idents: Vec<_> = extract_fields.iter().map(|(id, _)| id).collect();
    let extract_types: Vec<_> = extract_fields.iter().map(|(_, ty)| ty).collect();

    let init_from_parent = quote! {
        impl #impl_generics #name #type_generics #where_clause {
            /// Initializes the extracted context slices from the parent context.
            pub fn init_from_parent<P>(&self, parent_cx: &P)
            where
                #( #extract_types: for<'a> ::core::convert::From<&'a P>, )*
            {
                #(
                    let val = <#extract_types as ::core::convert::From<&P>>::from(parent_cx);
                    let _ = self.#extract_idents.set(val);
                )*
            }
        }
    };

    let collect_impl = if let Some(subcmd_id) = &subcommand_field {
        quote! {
            impl #impl_generics ::clifacet::prompt::Collect for #name #type_generics #where_clause {
                fn collect(
                    mut self,
                    session: &mut ::clifacet::prompt::Session,
                ) -> ::core::result::Result<Self, ::clifacet::Failure> {
                    self.#subcmd_id = ::clifacet::facet::__derive::collect_optional_tree(self.#subcmd_id, session)?;
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

    let tree_impl = quote! {
        impl #impl_generics ::clifacet::facet::__derive::Tree for #name #type_generics #where_clause {
            type Input = Self;
            fn into_input(self) -> ::core::result::Result<Self::Input, ::clifacet::Failure> {
                ::core::result::Result::Ok(self)
            }
        }
    };

    parsed.attrs.insert(
        0,
        syn::parse_quote!(#[derive(::clap::Args, ::std::clone::Clone, ::std::fmt::Debug)]),
    );

    Ok(quote! {
        #parsed
        #init_from_parent
        #collect_impl
        #tree_impl
    })
}

fn parse_group_args(attribute: TokenStream) -> syn::Result<bool> {
    if attribute.is_empty() {
        return Ok(false);
    }
    let entries =
        syn::parse::Parser::parse2(Punctuated::<Meta, Token![,]>::parse_terminated, attribute)?;
    for entry in entries {
        if entry.path().is_ident("commands") {
            return Ok(true);
        }
    }
    Ok(false)
}
