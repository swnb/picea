use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Data, DeriveInput, Fields, Ident, Token, Visibility,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum GetterMode {
    Ref,
    Copy,
    Clone,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SetterMode {
    Value,
    Into,
}

#[derive(Clone, Default)]
struct AccessorConfig {
    getter: Option<GetterMode>,
    setter: Option<SetterMode>,
    mutable: bool,
    visibility: Option<Visibility>,
    skip: bool,
}

enum AccessorOption {
    Get(GetterMode),
    Set(SetterMode),
    Mut,
    Vis(Visibility),
    Skip,
}

impl Parse for AccessorOption {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(Token![mut]) {
            input.parse::<Token![mut]>()?;
            return Ok(Self::Mut);
        }

        let ident = input.parse::<Ident>()?;

        match ident.to_string().as_str() {
            "get" => {
                if input.peek(token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    let mode = content.parse::<Ident>()?;
                    if !content.is_empty() {
                        return Err(syn::Error::new(content.span(), "unexpected getter options"));
                    }

                    return match mode.to_string().as_str() {
                        "copy" => Ok(Self::Get(GetterMode::Copy)),
                        "clone" => Ok(Self::Get(GetterMode::Clone)),
                        _ => Err(syn::Error::new(mode.span(), "expected `copy` or `clone`")),
                    };
                }

                Ok(Self::Get(GetterMode::Ref))
            }
            "set" => {
                if input.peek(token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    let mode = content.parse::<Ident>()?;
                    if !content.is_empty() {
                        return Err(syn::Error::new(content.span(), "unexpected setter options"));
                    }

                    return match mode.to_string().as_str() {
                        "into" => Ok(Self::Set(SetterMode::Into)),
                        _ => Err(syn::Error::new(mode.span(), "expected `into`")),
                    };
                }

                Ok(Self::Set(SetterMode::Value))
            }
            "skip" => Ok(Self::Skip),
            "vis" => {
                let content;
                parenthesized!(content in input);
                Ok(Self::Vis(content.parse()?))
            }
            _ => Err(syn::Error::new(ident.span(), "unsupported accessor option")),
        }
    }
}

pub fn macro_accessors(input: DeriveInput) -> TokenStream {
    let ident = input.ident;
    let generics = input.generics;
    let input_vis = input.vis;

    let struct_config = match parse_accessor_config(&input.attrs, false) {
        Ok(config) => config,
        Err(error) => return error.into_compile_error().into(),
    };

    let Data::Struct(data) = input.data else {
        return syn::Error::new(
            ident.span(),
            "Accessors can only be derived for structs with named fields",
        )
        .into_compile_error()
        .into();
    };

    let Fields::Named(named_fields) = data.fields else {
        return syn::Error::new(
            ident.span(),
            "Accessors can only be derived for structs with named fields",
        )
        .into_compile_error()
        .into();
    };

    let methods = named_fields.named.iter().map(|field| {
        let field_ident = field.ident.as_ref().expect("named field");
        let field_ty = &field.ty;

        let field_config = parse_accessor_config(&field.attrs, true);
        let field_config = match field_config {
            Ok(config) => merge_configs(&struct_config, &config),
            Err(error) => return error.into_compile_error(),
        };

        if field_config.skip {
            return quote!();
        }

        let method_vis = field_config
            .visibility
            .clone()
            .unwrap_or_else(|| input_vis.clone());

        let getter = field_config.getter.map(|mode| match mode {
            GetterMode::Ref => {
                quote! {
                    #method_vis fn #field_ident(&self) -> &#field_ty {
                        &self.#field_ident
                    }
                }
            }
            GetterMode::Copy => {
                quote! {
                    #method_vis fn #field_ident(&self) -> #field_ty {
                        self.#field_ident
                    }
                }
            }
            GetterMode::Clone => {
                quote! {
                    #method_vis fn #field_ident(&self) -> #field_ty {
                        self.#field_ident.clone()
                    }
                }
            }
        });

        let mutable = field_config.mutable.then(|| {
            let method_ident = Ident::new(&format!("{}_mut", field_ident), field_ident.span());
            quote! {
                #method_vis fn #method_ident(&mut self) -> &mut #field_ty {
                    &mut self.#field_ident
                }
            }
        });

        let setter = field_config.setter.map(|mode| {
            let method_ident = Ident::new(&format!("set_{}", field_ident), field_ident.span());
            match mode {
                SetterMode::Value => {
                    quote! {
                        #method_vis fn #method_ident(&mut self, value: #field_ty) {
                            self.#field_ident = value;
                        }
                    }
                }
                SetterMode::Into => {
                    quote! {
                        #method_vis fn #method_ident(&mut self, value: impl Into<#field_ty>) {
                            self.#field_ident = value.into();
                        }
                    }
                }
            }
        });

        quote! {
            #getter
            #mutable
            #setter
        }
    });

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            #(#methods)*
        }
    }
    .into()
}

fn merge_configs(defaults: &AccessorConfig, field: &AccessorConfig) -> AccessorConfig {
    AccessorConfig {
        getter: field.getter.or(defaults.getter),
        setter: field.setter.or(defaults.setter),
        mutable: defaults.mutable || field.mutable,
        visibility: field
            .visibility
            .clone()
            .or_else(|| defaults.visibility.clone()),
        skip: field.skip,
    }
}

fn parse_accessor_config(attrs: &[Attribute], allow_skip: bool) -> syn::Result<AccessorConfig> {
    let mut config = AccessorConfig::default();
    let mut errors: Option<syn::Error> = None;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("accessor")) {
        let result = parse_accessor_meta_list(attr, &mut config, allow_skip);

        if let Err(error) = result {
            if let Some(existing) = &mut errors {
                existing.combine(error);
            } else {
                errors = Some(error);
            }
        }
    }

    if config.skip
        && (config.getter.is_some()
            || config.setter.is_some()
            || config.mutable
            || config.visibility.is_some())
    {
        let error = syn::Error::new(
            attrs[0].span(),
            "`skip` cannot be combined with other accessor options",
        );
        if let Some(existing) = &mut errors {
            existing.combine(error);
        } else {
            errors = Some(error);
        }
    }

    if let Some(error) = errors {
        return Err(error);
    }

    Ok(config)
}

fn parse_accessor_meta_list(
    attr: &Attribute,
    config: &mut AccessorConfig,
    allow_skip: bool,
) -> syn::Result<()> {
    let items = attr.parse_args_with(Punctuated::<AccessorOption, Token![,]>::parse_terminated)?;

    for item in items {
        match item {
            AccessorOption::Get(mode) => {
                config.getter = Some(mode);
            }
            AccessorOption::Set(mode) => {
                config.setter = Some(mode);
            }
            AccessorOption::Mut => {
                config.mutable = true;
            }
            AccessorOption::Skip => {
                if !allow_skip {
                    return Err(syn::Error::new(
                        attr.span(),
                        "`skip` is only allowed on fields",
                    ));
                }
                config.skip = true;
            }
            AccessorOption::Vis(visibility) => {
                config.visibility = Some(visibility);
            }
        }
    }

    Ok(())
}
