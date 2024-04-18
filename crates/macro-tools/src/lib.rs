mod builder;
mod deref;
mod fields;

use builder::macro_builder;
use deref::macro_deref;
use fields::macro_fields;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parenthesized, parse_macro_input, punctuated::Punctuated, spanned::Spanned, Attribute, Data,
    DeriveInput, Ident, LitStr, Meta, Visibility,
};

#[proc_macro_derive(Shape, attributes(inner))]
pub fn shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let generics = input.generics;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote!(
        impl #impl_generics crate::collision::Collider for #ident #ty_generics #where_clause {}

        impl #impl_generics crate::element::SelfClone for #ident #ty_generics #where_clause {
            fn self_clone(&self) -> Box<dyn crate::element::ShapeTraitUnion> {
                self.clone().into()
            }
        }
    )
    .into()
}

#[proc_macro_derive(Deref, attributes(deref))]
pub fn deref(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_deref(input)
}

#[proc_macro_derive(Builder, attributes(default, builder, shared))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_builder(input)
}

#[proc_macro_derive(Fields, attributes(shared, r, w))]
pub fn fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_fields(input)
}

fn underscore_to_camelcase(input: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in input.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

#[proc_macro_attribute]
pub fn wasm_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::Item);

    let args_parsed =
        parse_macro_input!(attr with Punctuated::<Meta, syn::Token![,]>::parse_terminated);

    let mut bind_obj: Option<syn::Expr> = None;

    for arg in args_parsed {
        arg.path().is_ident("bind");
        let Meta::NameValue(meta) = arg else {
            return syn::Error::new(arg.span(), "bind can only use with bind = {{Obj}} ")
                .into_compile_error()
                .into();
        };

        let bind_object = meta.value;
        bind_obj = Some(bind_object);
    }

    let syn::Item::Struct(struct_data) = item else {
        return syn::Error::new(item.span(), "only apply to struct")
            .into_compile_error()
            .into();
    };

    let ident = struct_data.ident;

    let (impl_generics, ty_generics, where_clause) = struct_data.generics.split_for_impl();

    let fields: Vec<_> = struct_data
        .fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.clone();
            let ty = field.ty.clone();
            let field_name_str = field.ident.to_token_stream().to_string();
            let serde_field_name = underscore_to_camelcase(&field_name_str);

            let serde_field_name = LitStr::new(&serde_field_name, field_ident.span());
            quote!(
                #[serde(rename = #serde_field_name)]
                #field_ident: Option<#ty>,
            )
        })
        .collect();

    let bind_obj = bind_obj.map(|expr| {
        let bind_fields = struct_data.fields.iter().map(|field| {
            let field_ident = field.ident.clone();
            quote!(
                #field_ident: Some(target.#field_ident().into()),
            )
        });

        let target = expr;

        let builder_ident = Ident::new(
            &format!("{}Builder", target.to_token_stream()),
            target.span(),
        );

        let default_fields = struct_data.fields.iter().map(|field| {
            let field_ident = field.ident.clone();
            let default_expr = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("default"))
                .and_then(|attr| {
                    let Meta::NameValue(ref kv) = attr.meta else {
                        return None;
                    };

                    let value = kv.value.clone();
                    quote!(Some(#value)).into()
                })
                .unwrap_or(quote!(Some(Default::default())));

            quote!(
                #field_ident: #default_expr,
            )
        });

        let builder_fields = struct_data.fields.iter().map(|field| {
            let field_ident = field.ident.clone();
            let default_expr = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("default"))
                .and_then(|attr| {
                    let Meta::NameValue(ref kv) = attr.meta else {
                        return None;
                    };

                    Some(kv.value.clone())
                });

            match default_expr {
                Some(expr) => {
                    quote!(
                        .#field_ident(target.#field_ident.unwrap_or(#expr))
                    )
                }
                None => {
                    quote!(
                        .#field_ident(target.#field_ident.unwrap_or_default())
                    )
                }
            }
        });

        quote!(
            impl From<&picea::prelude::#target> for #ident {
                fn from(target: &picea::prelude::#target) -> Self {
                    Self {
                        #(#bind_fields)*
                    }
                }
            }

            impl From<&#ident> for picea::prelude::#builder_ident {
                fn from(target: &#ident) -> Self {
                    #builder_ident::new()
                        #(#builder_fields)*
                }
            }

            impl Default for #ident {
                fn default() -> Self {
                    Self {
                        #(#default_fields)*
                    }
                }
            }

        )
    });

    let attrs = struct_data.attrs;

    let web_config_ident = Ident::new(&format!("Web{}", ident), ident.span());
    let optional_web_config_ident = Ident::new(&format!("OptionalWeb{}", ident), ident.span());

    let field_valid_warning = {
        let field_valid_warning = format!("value of {} is not valid", ident);
        // TODO list all field and it's type
        field_valid_warning
    };

    let ident_str = format!("{ident}");

    let optional_ident_str = format!("{ident}Partial");

    let vis = struct_data.vis;

    quote!(
        #(#attrs)*
        #[derive(picea_macro_tools::Fields)]
        #[r]
        #[derive(Deserialize, Serialize)]
        #vis struct #impl_generics #ident #ty_generics #where_clause {
            #(#fields)*
        }

        impl TryInto<#ident> for #optional_web_config_ident {
            type Error = &'static str;
            fn try_into(self) -> Result<#ident, Self::Error> {
                let value: JsValue = self.into();
                let value: #ident = from_value(value).map_err(|_| {
                  #field_valid_warning
                })?;

                Ok(value)
            }
        }

        impl From<&#ident> for #web_config_ident {
            fn from(target: &#ident) -> #web_config_ident {
                serde_wasm_bindgen::to_value(&target).unwrap().into()
            }
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(typescript_type = #ident_str)]
            pub type #web_config_ident;

            #[wasm_bindgen(typescript_type = #optional_ident_str)]
            pub type #optional_web_config_ident;
        }

        #bind_obj
    )
    .into()
}
