use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Ident, Meta};

pub fn macro_builder(input: DeriveInput) -> TokenStream {
    let origin_ident = input.ident;
    let generics = input.generics;

    let vis = input.vis;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let Data::Struct(data) = input.data else {
        return syn::Error::new(origin_ident.span(), "error")
            .into_compile_error()
            .into();
    };

    let ident = Ident::new(&format!("{}Builder", origin_ident), origin_ident.span());

    let fields_iter = || {
        data.fields.iter().filter(|field| {
            field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("builder") || attr.path().is_ident("shared"))
                .map(|attr| {
                    let mut is_skip = false;
                    let _ = attr.parse_nested_meta(|meta| {
                        is_skip = meta.path.is_ident("skip");
                        Ok(())
                    });
                    !is_skip
                })
                .unwrap_or(true)
        })
    };

    let default_fields: Vec<_> = data
        .fields
        .iter()
        .map(|field| {
            let field_ident = &field.ident;
            let default_expr: Option<syn::Expr> = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("default"))
                .and_then(|attr| match &attr.meta {
                    Meta::Path(_) => None,
                    Meta::NameValue(meta) => Some(meta.value.clone()),
                    Meta::List(list) => list.parse_args().ok(),
                });

            match default_expr {
                Some(expr) => quote!(
                    #field_ident: #expr,
                ),
                None => quote!(
                    #field_ident: Default::default(),
                ),
            }
        })
        .collect();

    let fields: Vec<_> = data
        .fields
        .iter()
        .map(|field| {
            let field_ident = &field.ident;
            let ty = &field.ty;

            quote!(
                #field_ident: #ty,
            )
        })
        .collect();

    let build_fields: Vec<_> = data
        .fields
        .iter()
        .map(|field| {
            let field_ident = &field.ident;
            quote!(
                #field_ident: value.#field_ident,
            )
        })
        .collect();

    let property_methods: Vec<_> = fields_iter()
        .map(|field| {
            let field_ident = &field.ident;
            let ty = &field.ty;
            quote!(
                pub fn #field_ident(mut self, value: impl Into<#ty>) -> Self {
                    self.#field_ident = value.into();
                    self
                }
            )
        })
        .collect();

    quote!(
        #vis struct #ident {
            #(#fields)*
        }

        impl #impl_generics Default for #ident #ty_generics #where_clause {
            fn default() -> Self {
                Self {
                    #(#default_fields)*
                }
            }
        }

        impl #impl_generics Default for #origin_ident #ty_generics #where_clause {
            fn default() -> Self {
                Self {
                    #(#default_fields)*
                }
            }
        }

        impl #impl_generics From<#ident #ty_generics> for #origin_ident #ty_generics #where_clause {
            fn from(value: #ident #ty_generics) -> Self {
                Self {
                    #(#build_fields)*
                }
            }
        }

        impl #impl_generics #ident #ty_generics #where_clause {
            pub fn new() -> Self {
                Self::default()
            }

            #(#property_methods)*
        }
    )
    .into()
}
