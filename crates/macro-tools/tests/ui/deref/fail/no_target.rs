use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    value: String,
    label: &'static str,
}

fn main() {}
