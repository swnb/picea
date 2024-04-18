use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput};

pub fn macro_deref(input: DeriveInput) -> TokenStream {
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
