use picea_macro_tools::Builder;

#[derive(Builder)]
struct Meta {
    #[builder(shared)]
    value: String,
}

fn main() {}
