mod accessors;
mod builder;
mod deref;

use accessors::macro_accessors;
use builder::macro_builder;
use deref::macro_deref;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Deref, attributes(deref))]
pub fn deref(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_deref(input)
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_builder(input)
}

#[proc_macro_derive(Accessors, attributes(accessor))]
pub fn accessors(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    macro_accessors(input)
}
