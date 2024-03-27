use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Ident, Meta};

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

    let property_method: Vec<_> = fields_iter()
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

            #(#property_method)*
        }
    )
    .into()
}

#[proc_macro_derive(Fields, attributes(field, shared))]
pub fn fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let generics = input.generics;

    let vis = input.vis;

    let (readonly, writeable) =
        parse_attr_readonly_writeable(&input.attrs).unwrap_or((false, false));

    fn parse_attr_readonly_writeable(attrs: &[syn::Attribute]) -> Option<(bool, bool)> {
        let attr = attrs
            .iter()
            .find(|attr| attr.path().is_ident("field") || attr.path().is_ident("shared"))?;
        let mut readonly = false;
        let mut writeable = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("writeable") {
                writeable = true;
            }
            if meta.path.is_ident("readonly") {
                readonly = true;
            }
            Ok(())
        });
        Some((readonly, writeable))
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let Data::Struct(data) = input.data else {
        return syn::Error::new(ident.span(), "error")
            .into_compile_error()
            .into();
    };

    let property_method: Vec<_> = data
        .fields
        .iter()
        .map(|field| {
            let mut writeable = writeable;
            let mut readonly = readonly;

            if let Some((current_readonly, current_writeable)) =
                parse_attr_readonly_writeable(&field.attrs)
            {
                readonly = current_readonly;
                writeable = current_writeable;
            }

            let field_ident = field.ident.clone().unwrap();
            let ty = &field.ty;

            let primitive_types = [
                "bool", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128",
                "f32", "f64", "FloatNum",
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

            let readonly_field = readonly.then(|| {
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

            let filed_ident_mut = Ident::new(&format!("{}_mut", field_ident), field.ident.span());

            let writeable_field = writeable.then_some({
                quote!(
                    #vis fn #filed_ident_mut(&mut self) -> &mut #ty {
                        &mut self.#field_ident
                    }
                )
            });

            quote!(
                #readonly_field

                #writeable_field
            )
        })
        .collect();

    quote!(
        impl #impl_generics #ident #ty_generics #where_clause {
            #(#property_method)*
        }
    )
    .into()
}
