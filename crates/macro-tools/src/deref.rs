use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Data, DeriveInput, Fields, Ident, Meta, Token,
};

enum DerefOption {
    Mut,
}

impl Parse for DerefOption {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(Token![mut]) {
            input.parse::<Token![mut]>()?;
            return Ok(Self::Mut);
        }

        let ident = input.parse::<Ident>()?;
        Err(syn::Error::new(
            ident.span(),
            "unsupported deref option, expected `mut`",
        ))
    }
}

#[derive(Clone, Copy, Default)]
struct DerefConfig {
    mutable: bool,
}

struct DerefTarget {
    ident: Ident,
    ty: syn::Type,
    mutable: bool,
}

pub fn macro_deref(input: DeriveInput) -> TokenStream {
    let ident = input.ident;
    let generics = input.generics;

    let Data::Struct(data) = input.data else {
        return syn::Error::new(
            ident.span(),
            "Deref can only be derived for structs with named fields",
        )
        .into_compile_error()
        .into();
    };

    let Fields::Named(named_fields) = data.fields else {
        return syn::Error::new(
            ident.span(),
            "Deref can only be derived for structs with named fields",
        )
        .into_compile_error()
        .into();
    };

    // Keep the target-field scan explicit so diagnostics stay tied to the user's field.
    let mut deref_fields = Vec::new();
    let mut errors: Option<syn::Error> = None;

    for field in named_fields.named {
        let field_ident = field.ident.clone().expect("named field");

        match parse_deref_config(&field_ident, &field.attrs) {
            Ok(Some(config)) => deref_fields.push(DerefTarget {
                ident: field_ident,
                ty: field.ty,
                mutable: config.mutable,
            }),
            Ok(None) => {}
            Err(error) => {
                if let Some(existing) = &mut errors {
                    existing.combine(error);
                } else {
                    errors = Some(error);
                }
            }
        }
    }

    if let Some(error) = errors {
        return error.into_compile_error().into();
    }

    let deref_field = match deref_fields.len() {
        1 => deref_fields.pop().expect("one target field"),
        0 => {
            return syn::Error::new(
                ident.span(),
                "mark exactly one named field with `#[deref]` or `#[deref(mut)]`",
            )
            .into_compile_error()
            .into();
        }
        _ => {
            let mut error = syn::Error::new(
                deref_fields[1].ident.span(),
                "only one named field can be marked with `#[deref]`",
            );

            for field in deref_fields.iter().skip(2) {
                error.combine(syn::Error::new(
                    field.ident.span(),
                    "only one named field can be marked with `#[deref]`",
                ));
            }

            return error.into_compile_error().into();
        }
    };

    let deref_field_ident = deref_field.ident;
    let deref_field_ty = deref_field.ty;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let deref_mut_impl = deref_field.mutable.then(|| {
        quote! {
            impl #impl_generics core::ops::DerefMut for #ident #ty_generics #where_clause {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.#deref_field_ident
                }
            }
        }
    });

    quote!(
        impl #impl_generics core::ops::Deref for #ident #ty_generics #where_clause {
            type Target = #deref_field_ty;

            fn deref(&self) -> &Self::Target {
                &self.#deref_field_ident
            }
        }

        #deref_mut_impl
    )
    .into()
}

fn parse_deref_config(
    field_ident: &Ident,
    attrs: &[Attribute],
) -> syn::Result<Option<DerefConfig>> {
    let mut config = None;
    let mut errors: Option<syn::Error> = None;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("deref")) {
        match parse_deref_attr(attr) {
            Ok(parsed) => {
                if config.replace(parsed).is_some() {
                    let error = syn::Error::new(
                        field_ident.span(),
                        "duplicate `#[deref]` attribute on field",
                    );

                    if let Some(existing) = &mut errors {
                        existing.combine(error);
                    } else {
                        errors = Some(error);
                    }
                }
            }
            Err(error) => {
                if let Some(existing) = &mut errors {
                    existing.combine(error);
                } else {
                    errors = Some(error);
                }
            }
        }
    }

    if let Some(error) = errors {
        return Err(error);
    }

    Ok(config)
}

fn parse_deref_attr(attr: &Attribute) -> syn::Result<DerefConfig> {
    match &attr.meta {
        Meta::Path(_) => Ok(DerefConfig::default()),
        Meta::List(list) => {
            let mut config = DerefConfig::default();
            let options =
                list.parse_args_with(Punctuated::<DerefOption, Token![,]>::parse_terminated)?;

            if options.is_empty() {
                return Err(syn::Error::new(
                    list.span(),
                    "empty `#[deref()]` is invalid; use `#[deref]` or `#[deref(mut)]`",
                ));
            }

            if options.trailing_punct() {
                return Err(syn::Error::new(
                    list.span(),
                    "`#[deref(...)]` only supports the exact form `#[deref(mut)]`",
                ));
            }

            for option in options {
                match option {
                    DerefOption::Mut => {
                        if config.mutable {
                            return Err(syn::Error::new(
                                attr.span(),
                                "duplicate `mut` in `#[deref(...)]`",
                            ));
                        }

                        config.mutable = true;
                    }
                }
            }

            Ok(config)
        }
        Meta::NameValue(meta) => Err(syn::Error::new(
            meta.span(),
            "expected `#[deref]` or `#[deref(mut)]`",
        )),
    }
}
