use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref(clone)]
    value: String,
}

fn main() {}
