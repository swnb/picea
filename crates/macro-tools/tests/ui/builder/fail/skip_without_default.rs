use picea_macro_tools::Builder;

#[derive(Builder)]
struct Meta {
    #[builder(skip)]
    hidden: String,
}

fn main() {}
