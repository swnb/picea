struct A {}

struct ClosureWrapper {
    callback: Box<dyn FnOnce() -> A>,
}

impl Into<A> for ClosureWrapper {
    fn into(self) -> A {
        (self.callback)()
    }
}

fn main() {}
