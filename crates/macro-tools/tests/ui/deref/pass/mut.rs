use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref(mut)]
    value: Vec<i32>,
    label: &'static str,
}

fn main() {
    let mut wrapper = Wrapper {
        value: vec![1, 2],
        label: "meta",
    };

    wrapper.push(3);

    assert_eq!(wrapper.as_slice(), &[1, 2, 3]);
    assert_eq!(wrapper.label, "meta");
}
