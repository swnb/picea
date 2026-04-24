use picea_macro_tools::Builder;

#[derive(Builder)]
struct Meta {
    name: String,
    #[builder(default)]
    retries: usize,
    #[builder(default = String::from("debug"))]
    profile: String,
    #[builder(skip, default = Vec::new())]
    cache: Vec<String>,
}

fn main() {
    let meta = MetaBuilder::new().name("hello").build().unwrap();
    assert_eq!(meta.name, "hello");
    assert_eq!(meta.retries, 0);
    assert_eq!(meta.profile, "debug");
    assert!(meta.cache.is_empty());
}
