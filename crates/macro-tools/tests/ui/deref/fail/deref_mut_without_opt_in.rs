use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref]
    value: Vec<i32>,
    label: &'static str,
}

#[allow(unused_mut)]
fn main() {
    let mut wrapper = Wrapper {
        value: vec![1, 2],
        label: "meta",
    };

    wrapper.push(3);
}
