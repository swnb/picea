use picea_macro_tools::Accessors;

#[derive(Accessors)]
struct Meta {
    #[accessor(get, set)]
    name: String,
}

fn main() {
    let mut meta = Meta {
        name: String::from("hello"),
    };

    let _ = meta.set_name(String::from("world")).name();
}
