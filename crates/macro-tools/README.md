# Macro Tools

contain some useful macro tools;

## Field

create field read and write method for field

for example

create read field for Meta;

```rust
    #[derive(Fields)]
    #[r]
    struct Meta {
        field_a: String,
        field_b: i32,
    }

    let meta = Meta {
        field_a: String::new(),
        field_b: 3,
    };

    let field_a: &String = meta.field_a();
    let field_b: i32 = meta.field_b();
```

create common write field for struct

```rust
    #[derive(Fields)]
    #[w]
    struct Meta {
        field_a: String,
        field_b: i32,
    }

    let mut meta = Meta {
        field_a: String::new(),
        field_b: 3,
    };

    let field_a_mut: &mut String = meta.field_a_mut();
    let field_b_mut: &mut i32 = meta.field_b_mut();
```

custom write field for struct with reducer: `impl FnOnce(Type) -> Type`

```rust
    #[derive(Fields)]
    #[r]
    struct Meta {
        #[w(reducer)]
        field_a: String,
        field_b: i32,
    }

    let mut meta = Meta {
        field_a: String::new(),
        field_b: 3,
    };

    let field_b = meta.field_b();

    meta.set_field_a(|field_a| field_a + &field_b.to_string());
```

custom write field for struct with `set`

```rust
    #[derive(Fields)]
    #[r]
    struct Meta {
        #[w(set)]
        field_a: String,
        field_b: i32,
    }

    let mut meta = Meta {
        field_a: String::new(),
        field_b: 3,
    };

    let field_b = meta.field_b();

    meta.set_field_a(meta.field_a().clone() + &field_b.to_string());
```

visibility of Fields

if struct is defined as `pub` then all field method will be `pub`

if struct is defined as `private` then all field method will be `private`

or custom `vis` for field method

```rust
mod private {
    use picea_macro_tools::Fields;

    #[derive(Fields)]
    #[r]
    pub struct Meta {
        #[w(set, vis(pub(self)))]
        field_a: String,
        field_b: i32,
    }
}

/// follow code will not compile
let mut meta = private::Meta::new();

meta.set_field_a(""); // function `set_field_a` is private
```
