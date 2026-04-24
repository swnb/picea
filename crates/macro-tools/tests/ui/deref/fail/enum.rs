use picea_macro_tools::Deref;

#[derive(Deref)]
enum Wrapper {
    Value {
        #[deref]
        value: String,
    },
}

fn main() {}
