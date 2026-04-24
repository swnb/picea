use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref]
    first: String,
    #[deref(mut)]
    second: String,
}

fn main() {}
