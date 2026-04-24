use picea_macro_tools::Accessors;

#[derive(Accessors)]
struct Meta {
    #[accessor(get, skip)]
    name: String,
}

fn main() {}
