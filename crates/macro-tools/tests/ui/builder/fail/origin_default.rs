use picea_macro_tools::Builder;

#[derive(Builder)]
struct Meta {
    name: String,
}

fn main() {
    let _ = Meta::default();
}
