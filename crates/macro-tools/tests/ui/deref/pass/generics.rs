use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper<'a, T>
where
    T: AsRef<str>,
{
    #[deref]
    value: T,
    label: &'a str,
}

fn main() {
    let wrapper = Wrapper {
        value: String::from("picea"),
        label: "meta",
    };

    assert_eq!(wrapper.as_str(), "picea");
    assert_eq!(wrapper.label, "meta");
}
