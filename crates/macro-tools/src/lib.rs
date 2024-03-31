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
    let ident = input.ident;
    let generics = input.generics;
    let Data::Struct(data) = input.data else {
        return syn::Error::new(ident.span(), "Deref can only be applied to structs")
            .into_compile_error()
            .into();
    };

    let mut deref_field: Option<(syn::Ident, syn::Type)> = None;

    for field in data.fields {
        for attr in field.attrs.iter() {
            if attr.path().is_ident("deref") {
                deref_field = Some((field.ident.clone().unwrap(), field.ty.clone()));
            }
        }
    }

    let Some((deref_field_ident, deref_field_ty)) = deref_field else {
        return syn::Error::new(
            ident.span(),
            "must set one deref field when use Deref macro",
        )
        .into_compile_error()
        .into();
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote!(
        impl #impl_generics core::ops::Deref for #ident #ty_generics #where_clause {
            type Target = #deref_field_ty;
            fn deref(&self) -> &Self::Target {
                &self.#deref_field_ident
            }
        }

        impl #impl_generics core::ops::DerefMut for #ident #ty_generics #where_clause {
           fn deref_mut(&mut self) -> &mut Self::Target {
             &mut self.#deref_field_ident
           }
        }
    )
    .into()
}

#[proc_macro_derive(Builder, attributes(default, builder, shared))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
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

#[proc_macro_derive(Fields, attributes(shared, r, w))]
pub fn fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
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

    let parse_attr_read = |attrs: &[syn::Attribute]| -> Option<Visibility> {
        find_attr(attrs, "r").map(|attr| {
            let mut field_vis: Visibility = input_vis.clone();
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("vis") {
                    let content;
                    parenthesized!(content in meta.input);
                    eprintln!("{}", content.to_string());
                    let value = content.parse::<Visibility>()?;
                    field_vis = value;
                }
                Ok(())
            });

            field_vis
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
                    eprintln!("{}", content.to_string());
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
                .map(|vis| {
                    if should_return_copy_when_read {
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
                            self.#field_ident = reducer(self.#field_ident);
                            self
                        }
                    )
                }else{
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

            impl #ident {
                // heavy copy
                pub fn transform_to_origin_bind_struct(&self) -> picea::prelude::#target {
                    let builder: picea::prelude::#builder_ident = self.into();
                    builder.into()
                }
            }
        )
    });

    let attrs = struct_data.attrs;

    let web_config_ident = Ident::new(&format!("Web{}", ident), ident.span());
    let optional_web_config_ident = Ident::new(&format!("OptionalWeb{}", ident), ident.span());

    let field_valid_warning = {
        let mut field_valid_warning = format!("value of {} is not valid", ident);
        // TODO list all field and it's type
        field_valid_warning
    };

    let ident_str = format!("{ident}");

    let optional_ident_str = format!("{ident}Partial");

    let vis = struct_data.vis;

    quote!(
        #(#attrs)*
        #[derive(macro_tools::Fields)]
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
