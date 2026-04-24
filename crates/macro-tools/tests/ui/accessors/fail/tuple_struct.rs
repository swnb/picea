use picea_macro_tools::Accessors;

#[derive(Accessors)]
struct Meta(#[accessor(get)] String);

fn main() {}
