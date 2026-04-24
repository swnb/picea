use picea_macro_tools::Accessors;

#[derive(Accessors)]
struct Meta {
    #[accessor(skip, vis(pub(crate)))]
    hidden: String,
}

fn main() {}
