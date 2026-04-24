use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref()]
    value: String,
}

fn main() {}
