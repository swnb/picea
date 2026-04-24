# Macro Tools

`crates/macro-tools` is a standalone proc-macro crate in the Picea workspace. It is verified independently and is not currently part of `crates/picea`'s direct dependency graph.

It currently exposes three derive helpers:

- `Accessors`: generate explicit field access methods.
- `Builder`: generate a small companion builder type.
- `Deref`: forward `Deref` and optional `DerefMut` to one field.

The crate is intentionally narrow after the reset. Historical `Fields`, `Shape`, `wasm_config`, `r`, `w`, `shared`, and reducer behavior is not retained as a compatibility layer.

## Validation

Run the crate-local test suite with:

```sh
rtk proxy cargo test -p picea-macro-tools
```

The suite includes `trybuild` fixtures for both accepted macro forms and compile-fail diagnostics.

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

Generated method names:

| Option | Generated method | Return or argument |
| --- | --- | --- |
| `get` | `field()` | `&T` |
| `get(copy)` | `field()` | `T`; the field type must be `Copy` |
| `get(clone)` | `field()` | `T`; the field type must be `Clone` |
| `mut` | `field_mut()` | `&mut T` |
| `set` | `set_field(value)` | accepts `T` |
| `set(into)` | `set_field(value)` | accepts `impl Into<T>` |

Additional options:

| Option | Meaning |
| --- | --- |
| `vis(...)` | Overrides generated method visibility, for example `vis(pub(crate))`. |
| `skip` | Field-only. Generates no accessor methods for that field. Cannot be combined with any other accessor option. |

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

/// The following code will not compile:
let mut meta = private::Meta::new();

meta.set_field_a(String::new()); // function `set_field_a` is private
```

Current limitations:

- Only named-field structs are supported.
- Enums, tuple structs, and unit structs are rejected.
- Legacy `#[r]`, `#[w]`, `#[shared]`, and reducer options are rejected.
- `get(copy)` and `get(clone)` do not add trait bounds for you. If the field type is not `Copy` or `Clone`, the generated implementation fails with the normal Rust type error at the field access.

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

Generated API:

| Generated item | Meaning |
| --- | --- |
| `SettingsBuilder` | Builder type named by appending `Builder` to the source struct. |
| `SettingsBuilder::new()` | Creates a builder with every field unset. |
| `builder.field(value)` | Chainable setter for required and defaulted fields. Setters accept `impl Into<FieldType>`. |
| `builder.build()` | Returns `Ok(Settings)` or `Err("missing field: field_name")`. |

Supported builder options:

| Option | Meaning |
| --- | --- |
| `default` | Use `Default::default()` if the field was not set. |
| `default = ...` | Use the given expression if the field was not set. The expression is evaluated during `build()`, not during `new()`. |
| `skip` | Do not generate a setter. Must be paired with `default` or `default = ...`. |

Current limitations:

- Only named-field structs are supported.
- Struct-level `#[builder(...)]` attributes are rejected.
- The derive does not generate `Default` for the original struct or for the builder type.
- Fields named `new` or `build` are rejected because they collide with generated methods.
- The old `shared` behavior is removed.

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

Generated API:

| Attribute | Generated impls |
| --- | --- |
| `#[deref]` | `impl core::ops::Deref<Target = FieldType>` |
| `#[deref(mut)]` | `impl core::ops::Deref<Target = FieldType>` and `impl core::ops::DerefMut` |

Current rules:

- mark exactly one named field with `#[deref]` or `#[deref(mut)]`
- `#[deref(mut)]` is the only supported option form
- tuple structs, unit structs, and enums are rejected
- duplicate `#[deref]` attributes, empty `#[deref()]`, and multiple target fields are rejected
