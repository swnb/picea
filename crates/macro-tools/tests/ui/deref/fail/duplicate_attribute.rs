use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref]
    #[deref]
    value: String,
}

fn main() {}
