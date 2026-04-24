use picea_macro_tools::Accessors;

#[derive(Accessors)]
#[accessor(skip)]
struct Meta {
    name: String,
}

fn main() {}
