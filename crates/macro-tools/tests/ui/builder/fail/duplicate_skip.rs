use picea_macro_tools::Builder;

#[derive(Builder)]
struct Meta {
    #[builder(skip, skip, default)]
    value: String,
}

fn main() {}
