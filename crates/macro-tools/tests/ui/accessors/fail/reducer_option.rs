use picea_macro_tools::Accessors;

#[derive(Accessors)]
struct Meta {
    #[accessor(reducer)]
    name: String,
}

fn main() {}
