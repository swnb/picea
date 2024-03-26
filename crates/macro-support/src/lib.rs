use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

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
