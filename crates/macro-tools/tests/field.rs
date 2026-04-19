use picea_macro_tools::Fields;

#[test]
fn test_common_read_field() {
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

    assert_eq!(meta.field_a(), "");
    assert_eq!(meta.field_b(), 3);
}

#[test]
fn test_common_write_field() {
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

    meta.field_a_mut().push_str("updated");
    *meta.field_b_mut() = 4;

    assert_eq!(meta.field_a, "updated");
    assert_eq!(meta.field_b, 4);
}

#[test]
fn test_custom_write_field() {
    #[derive(Fields)]
    #[r]
    struct Meta {
        #[w]
        field_a: String,
        field_b: i32,
    }

    let mut meta = Meta {
        field_a: String::new(),
        field_b: 3,
    };

    let field_b = meta.field_b();
    let field_a_mut: &mut String = meta.field_a_mut();
    *field_a_mut = field_b.to_string();
}

#[test]
fn test_custom_write_field_reducer() {
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
}

#[test]
fn test_custom_write_field_reduce() {
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
}

mod private {
    use picea_macro_tools::Fields;

    #[derive(Fields)]
    #[r]
    pub(crate) struct Meta {
        #[w(set)]
        field_a: String,
        field_b: i32,
    }

    pub static mut META: Meta = Meta::new();

    impl Meta {
        pub const fn new() -> Self {
            Meta {
                field_a: String::new(),
                field_b: 3,
            }
        }
    }
}

#[test]
fn test_custom_write_field_vis() {
    let meta = std::ptr::addr_of_mut!(private::META);
    unsafe {
        (*meta).set_field_a("");
        assert_eq!((*meta).field_a(), "");
        assert_eq!((*meta).field_b(), 3);
    }
}
