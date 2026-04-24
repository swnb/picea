use picea_macro_tools::Builder;

#[derive(Builder)]
struct Meta {
    #[builder(default, default)]
    value: String,
}

fn main() {}
