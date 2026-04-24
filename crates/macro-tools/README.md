# Macro Tools

`crates/macro-tools` is a standalone proc-macro crate in the Picea workspace. It is verified independently and is not currently part of `crates/picea`'s direct dependency graph.

It currently contains derive helpers such as `Accessors`, `Builder`, and `Deref`.

## Accessors

`Accessors` derives getter/setter helpers for named struct fields.

```rust
use picea_macro_tools::Accessors;

#[derive(Accessors)]
#[accessor(get, set, vis(pub(crate)))]
struct Meta {
    #[accessor(set(into), mut)]
    field_a: String,
    #[accessor(get(copy))]
    field_b: i32,
    #[accessor(get(clone))]
    field_c: String,
    #[accessor(skip)]
    hidden: bool,
}

let mut meta = Meta {
    field_a: String::new(),
    field_b: 3,
    field_c: String::from("payload"),
    hidden: true,
};

assert_eq!(meta.field_a(), "");
assert_eq!(meta.field_b(), 3);
assert_eq!(meta.field_c(), "payload");
meta.set_field_a("updated");
meta.field_a_mut().push('!');
assert_eq!(meta.field_a(), "updated!");
```

Struct-level `#[accessor(...)]` settings provide defaults, and field-level `#[accessor(...)]` settings override them.

Supported accessor options in the current crate include:

- `get`
- `get(copy)`
- `get(clone)`
- `set`
- `set(into)`
- `mut`
- `vis(...)`
- `skip`

Visibility follows the struct by default, and `vis(...)` lets you override it per struct or field.

```rust
mod private {
    use picea_macro_tools::Accessors;

    #[derive(Accessors)]
    #[accessor(get)]
    pub struct Meta {
        #[accessor(set, vis(pub(self)))]
        field_a: String,
        field_b: i32,
    }

    impl Meta {
        pub fn new() -> Self {
            Self {
                field_a: String::new(),
                field_b: 3,
            }
        }
    }
}

/// follow code will not compile
let mut meta = private::Meta::new();

meta.set_field_a(String::new()); // function `set_field_a` is private
```

## Builder

`Builder` derives a `StructNameBuilder` with `new()`, setter methods, and `build() -> Result<StructName, &'static str>`.

```rust
use picea_macro_tools::Builder;

#[derive(Debug, PartialEq, Builder)]
struct Settings {
    name: String,
    #[builder(default)]
    retries: usize,
    #[builder(default = String::from("stable"))]
    profile: String,
    #[builder(skip, default = Vec::new())]
    cached: Vec<String>,
}

let settings = SettingsBuilder::new().name("picea").build().unwrap();

assert_eq!(settings.name, "picea");
assert_eq!(settings.retries, 0);
assert_eq!(settings.profile, "stable");
assert!(settings.cached.is_empty());
```

Supported builder options in the current crate include:

- `default`
- `default = ...`
- `skip` (requires `default` or `default = ...`)

`Builder` currently supports named structs and generates setters that accept `Into<FieldType>`.

## Deref

`Deref` derives `core::ops::Deref` for exactly one named field. Use `#[deref(mut)]` when the wrapper should also implement `DerefMut`.

```rust
use picea_macro_tools::Deref;

#[derive(Deref)]
struct Wrapper {
    #[deref(mut)]
    value: Vec<i32>,
    label: &'static str,
}

let mut wrapper = Wrapper {
    value: vec![1, 2],
    label: "meta",
};

wrapper.push(3);

assert_eq!(wrapper.as_slice(), &[1, 2, 3]);
assert_eq!(wrapper.label, "meta");
```

Current `Deref` rules:

- mark exactly one named field with `#[deref]` or `#[deref(mut)]`
- `#[deref(mut)]` is the only supported option form
- tuple structs, unit structs, and enums are rejected
