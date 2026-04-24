use picea_macro_tools::Accessors;

#[derive(Accessors)]
struct Meta {
    #[field(get)]
    name: String,
}

fn main() {}
