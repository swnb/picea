use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper(#[deref] String);

fn main() {}
