use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Shape, attributes(inner))]
pub fn shape(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let generics = input.generics;
    quote!(
        impl<#generics> crate::collision::Collider for #ident<#generics> {}

        impl<#generics> crate::element::SelfClone for #ident<#generics> {
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

    quote!(
        impl<#generics> core::ops::Deref for #ident<#generics> {
            type Target = #deref_field_ty;
            fn deref(&self) -> &Self::Target {
                &self.#deref_field_ident
            }
        }

        impl<#generics> core::ops::DerefMut for #ident<#generics> {
           fn deref_mut(&mut self) -> &mut Self::Target {
             &mut self.#deref_field_ident
           }
        }
    )
    .into()
}
