use picea_macro_tools::Accessors;

#[derive(Accessors)]
#[accessor(get, set, vis(pub(crate)))]
struct Meta<T>
where
    T: Clone,
{
    #[accessor(set(into), mut)]
    name: String,
    #[accessor(get(copy))]
    count: usize,
    #[accessor(get(clone))]
    payload: T,
    #[accessor(skip)]
    hidden: bool,
}

fn takes_ref(_: &String) {}
fn takes_copy(_: usize) {}
fn takes_clone(_: String) {}
fn takes_mut(_: &mut String) {}
fn takes_unit(_: ()) {}

fn main() {
    let mut meta = Meta {
        name: String::from("hello"),
        count: 3,
        payload: String::from("payload"),
        hidden: true,
    };

    takes_ref(meta.name());
    takes_copy(meta.count());
    takes_clone(meta.payload());
    takes_mut(meta.name_mut());
    takes_unit(meta.set_count(7));
    takes_unit(meta.set_name("world"));
    let _ = meta.hidden;
}
