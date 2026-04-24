use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref]
    value: String,
    label: &'static str,
}

fn main() {
    let wrapper = Wrapper {
        value: String::from("picea"),
        label: "meta",
    };

    assert_eq!(wrapper.len(), 5);
    assert_eq!(&*wrapper, "picea");
    assert_eq!(wrapper.label, "meta");
}
