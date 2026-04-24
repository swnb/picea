use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref = "mut"]
    value: String,
}

fn main() {}
