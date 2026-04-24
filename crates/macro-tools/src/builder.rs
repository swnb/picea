use proc_macro::TokenStream;
use quote::{__private::TokenStream as TokenStream2, format_ident, quote};
use syn::{spanned::Spanned, Data, DeriveInput, Expr, Field, Fields, Ident, Result, Token};

pub fn macro_builder(input: DeriveInput) -> TokenStream {
    match expand_builder(input) {
        Ok(stream) => stream.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_builder(input: DeriveInput) -> Result<TokenStream2> {
    if let Some(attr) = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("builder"))
    {
        return Err(syn::Error::new(
            attr.span(),
            "builder attributes are only supported on fields",
        ));
    }

    let origin_ident = input.ident;
    let builder_ident = format_ident!("{}Builder", origin_ident);
    let generics = input.generics;
    let vis = input.vis;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let Data::Struct(data) = input.data else {
        return Err(syn::Error::new(
            origin_ident.span(),
            "Builder can only be derived for structs with named fields",
        ));
    };

    let Fields::Named(fields) = data.fields else {
        return Err(syn::Error::new(
            origin_ident.span(),
            "Builder can only be derived for structs with named fields",
        ));
    };

    let fields: Result<Vec<_>> = fields.named.iter().map(parse_builder_field).collect();
    let fields = fields?;

    let builder_fields = fields.iter().map(BuilderField::builder_field);
    let builder_new_fields = fields.iter().map(BuilderField::builder_initializer);
    let builder_setters = fields.iter().filter_map(BuilderField::setter);
    let build_fields = fields.iter().map(BuilderField::build_field);

    Ok(quote! {
        #vis struct #builder_ident #generics {
            #(#builder_fields)*
        }

        impl #impl_generics #builder_ident #ty_generics #where_clause {
            pub fn new() -> Self {
                Self {
                    #(#builder_new_fields)*
                }
            }

            #(#builder_setters)*

            pub fn build(self) -> ::core::result::Result<#origin_ident #ty_generics, &'static str> {
                ::core::result::Result::Ok(#origin_ident {
                    #(#build_fields)*
                })
            }
        }
    })
}

#[derive(Clone)]
enum FieldMode {
    Required,
    Default(Expr),
    Skip(Expr),
}

struct BuilderField {
    ident: Ident,
    ty: syn::Type,
    mode: FieldMode,
}

impl BuilderField {
    fn builder_field(&self) -> TokenStream2 {
        let ident = &self.ident;
        let ty = &self.ty;

        quote! {
            #ident: ::core::option::Option<#ty>,
        }
    }

    fn builder_initializer(&self) -> TokenStream2 {
        let ident = &self.ident;

        quote! {
            #ident: ::core::option::Option::None,
        }
    }

    fn setter(&self) -> Option<TokenStream2> {
        let ident = &self.ident;
        let ty = &self.ty;

        match self.mode {
            FieldMode::Skip(_) => None,
            FieldMode::Required => Some(quote! {
                pub fn #ident(mut self, value: impl ::core::convert::Into<#ty>) -> Self {
                    self.#ident = ::core::option::Option::Some(value.into());
                    self
                }
            }),
            FieldMode::Default(_) => Some(quote! {
                pub fn #ident(mut self, value: impl ::core::convert::Into<#ty>) -> Self {
                    self.#ident = ::core::option::Option::Some(value.into());
                    self
                }
            }),
        }
    }

    fn build_field(&self) -> TokenStream2 {
        let ident = &self.ident;

        match &self.mode {
            FieldMode::Required => {
                quote! {
                    #ident: match self.#ident {
                        ::core::option::Option::Some(value) => value,
                        ::core::option::Option::None => {
                            return ::core::result::Result::Err(concat!("missing field: ", stringify!(#ident)));
                        }
                    },
                }
            }
            FieldMode::Default(expr) | FieldMode::Skip(expr) => {
                quote! {
                    #ident: match self.#ident {
                        ::core::option::Option::Some(value) => value,
                        ::core::option::Option::None => #expr,
                    },
                }
            }
        }
    }
}

// Builder field parsing stays centralized so the derive keeps a single source
// of truth for supported field options and their validation rules.
fn parse_builder_field(field: &Field) -> Result<BuilderField> {
    let ident = field.ident.clone().ok_or_else(|| {
        syn::Error::new(
            field.span(),
            "Builder can only be derived for structs with named fields",
        )
    })?;

    if ident == "new" || ident == "build" {
        return Err(syn::Error::new(
            ident.span(),
            format!(
                "`{ident}` is reserved by Builder; rename the field to avoid colliding with generated methods"
            ),
        ));
    }

    let mut skip = false;
    let mut default_expr = None;

    for attr in field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("builder"))
    {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                if skip {
                    return Err(meta.error("duplicate `skip` builder option"));
                }
                skip = true;
                return Ok(());
            }

            if meta.path.is_ident("default") {
                if default_expr.is_some() {
                    return Err(meta.error("duplicate `default` builder option"));
                }

                if meta.input.peek(Token![=]) {
                    let _: Token![=] = meta.input.parse()?;
                    let expr = meta.input.parse()?;
                    default_expr = Some(expr);
                } else {
                    default_expr = Some(syn::parse_quote!(::core::default::Default::default()));
                }

                return Ok(());
            }

            Err(meta.error("unsupported builder option"))
        })?;
    }

    if skip && default_expr.is_none() {
        return Err(syn::Error::new(
            ident.span(),
            "`#[builder(skip)]` requires `#[builder(default)]` or `#[builder(default = ...)]`",
        ));
    }

    let mode = match (skip, default_expr) {
        (true, Some(expr)) => FieldMode::Skip(expr),
        (false, Some(expr)) => FieldMode::Default(expr),
        (false, None) => FieldMode::Required,
        (true, None) => unreachable!("skip fields without defaults are rejected above"),
    };

    Ok(BuilderField {
        ident,
        ty: field.ty.clone(),
        mode,
    })
}
