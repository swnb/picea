use picea_macro_tools::Accessors;

#[derive(Accessors)]
#[accessor(get, vis(pub(crate)))]
struct Meta<T>
where
    T: Clone,
{
    #[accessor(set(into), mut)]
    name: String,
    #[accessor(get(clone))]
    payload: T,
}

fn main() {
    let mut meta = Meta {
        name: String::from("hello"),
        payload: String::from("payload"),
    };

    assert_eq!(meta.name(), "hello");
    meta.set_name("world");
    meta.name_mut().push('!');
    assert_eq!(meta.name(), "world!");
    assert_eq!(meta.payload(), "payload");
}
