use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parenthesized, spanned::Spanned, Attribute, Data, DeriveInput, Ident, LitStr, Meta, Visibility,
};

pub fn macro_fields(input: DeriveInput) -> TokenStream {
    let ident = input.ident;
    let generics = input.generics;

    let input_vis = input.vis;

    fn find_attr<'a>(attrs: &'a [syn::Attribute], ident: &str) -> Option<&'a Attribute> {
        attrs.iter().find(|attr| attr.path().is_ident(ident))
    }

    let should_skip = |attrs: &[syn::Attribute]| -> bool {
        attrs
            .iter()
            .filter(|attr| ["shared", "r", "w"].iter().any(|k| attr.path().is_ident(k)))
            .any(|attr| {
                let mut is_skip = false;
                let _ = attr.parse_nested_meta(|meta| {
                    is_skip = meta.path.is_ident("skip");
                    Ok(())
                });
                is_skip
            })
    };

    let parse_attr_read = |attrs: &[syn::Attribute]| -> Option<(Visibility, bool)> {
        find_attr(attrs, "r").map(|attr| {
            let mut field_vis: Visibility = input_vis.clone();
            let mut auto_copy = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("vis") {
                    let content;
                    parenthesized!(content in meta.input);
                    let value = content.parse::<Visibility>()?;
                    field_vis = value;
                } else if meta.path.is_ident("copy") {
                    auto_copy = true
                }
                Ok(())
            });

            (field_vis, auto_copy)
        })
    };

    let parse_attr_write = |attrs: &[syn::Attribute]| -> Option<(bool, bool, Visibility)> {
        find_attr(attrs, "w").map(|attr| {
            let mut field_reducer: bool = false;
            let mut field_set: bool = false;
            let mut field_vis: Visibility = input_vis.clone();
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("reducer") {
                    field_reducer = true;
                    field_set = true;
                }
                if meta.path.is_ident("set") {
                    field_set = true;
                }

                if meta.path.is_ident("vis") {
                    let content;
                    parenthesized!(content in meta.input);
                    let value = content.parse::<Visibility>()?;
                    field_vis = value;
                }
                Ok(())
            });
            (field_reducer, field_set, field_vis)
        })
    };

    let global_attr_read = parse_attr_read(&input.attrs);

    let global_attr_write = parse_attr_write(&input.attrs);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let Data::Struct(data) = input.data else {
        return syn::Error::new(ident.span(), "error")
            .into_compile_error()
            .into();
    };

    let property_method = data
        .fields
        .iter()
        .filter(|field| !should_skip(&field.attrs))
        .map(|field| {
            let field_ident = field.ident.clone().unwrap();
            let ty = field.ty.clone();

            let primitive_types = [
                "bool", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128",
                "f32", "f64", "FloatNum", "ID",
            ];

            let should_return_copy_when_read = match &field.ty {
                syn::Type::Path(path) => {
                    let t = path.into_token_stream().to_string();
                    primitive_types
                        .iter()
                        .any(|primitive_type| primitive_type == &t)
                }
                _ => false,
            };
            let read_field_method = parse_attr_read(&field.attrs)
                .or(global_attr_read.clone())
                .map(|(vis, auto_copy)| {
                    if should_return_copy_when_read || auto_copy {
                        quote!(
                            #vis fn #field_ident(&self) -> #ty {
                                self.#field_ident
                            }
                        )
                    } else {
                        quote!(
                            #vis fn #field_ident(&self) -> &#ty {
                                &self.#field_ident
                            }
                        )
                    }
                });

            let write_field_method = parse_attr_write(&field.attrs)
                .or(global_attr_write.clone())
                .map(|(use_reducer, use_set_prefix_method, vis)| {
                    if use_set_prefix_method {
                        let set_field_ident =
                            Ident::new(&format!("set_{}", field_ident), field.ident.span());
                            if use_reducer {
                        quote!(
                            #vis fn #set_field_ident(&mut self, mut reducer: impl FnOnce(#ty)-> #ty) -> &mut Self {
                                self.#field_ident = reducer(core::mem::take(&mut self.#field_ident));
                                self
                            }
                        )
                    } else {
                        quote!(
                            #vis fn #set_field_ident(&mut self, value: impl Into<#ty>) -> &mut Self {
                                self.#field_ident = value.into();
                                self
                            }
                        )
                        }
                    } else {
                        let filed_ident_mut =
                            Ident::new(&format!("{}_mut", field_ident), field.ident.span());

                        quote!(
                            #vis fn #filed_ident_mut(&mut self) -> &mut #ty {
                                &mut self.#field_ident
                            }
                        )
                    }
                });


            quote!(
                #read_field_method

                #write_field_method
            )
        });

    quote!(
        impl #impl_generics #ident #ty_generics #where_clause {
            #(#property_method)*
        }
    )
    .into()
}
