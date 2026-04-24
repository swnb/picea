use picea_macro_tools::Builder;

#[derive(Builder)]
struct Meta<T>
where
    T: Clone,
{
    payload: T,
    #[builder(default)]
    tags: Vec<String>,
    #[builder(skip, default = 9)]
    priority: usize,
}

fn main() {
    let meta = MetaBuilder::<String>::new()
        .payload("payload")
        .tags(vec![String::from("tag")])
        .build()
        .unwrap();

    assert_eq!(meta.payload, "payload");
    assert_eq!(meta.tags, vec![String::from("tag")]);
    assert_eq!(meta.priority, 9);
}
