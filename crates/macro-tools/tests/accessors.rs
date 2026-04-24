use picea_macro_tools::Accessors;

#[test]
fn runtime_generates_accessors() {
    fn expect_ref(_: &String) {}
    fn expect_copy(_: i32) {}
    fn expect_clone(_: String) {}
    fn expect_mut_ref(_: &mut String) {}

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

    expect_ref(meta.field_a());
    expect_copy(meta.field_b());
    expect_clone(meta.field_c());
    assert_eq!(meta.field_a(), "");
    assert_eq!(meta.field_b(), 3);
    assert_eq!(meta.field_c(), "payload");
    meta.set_field_b(7);
    assert_eq!(meta.field_b(), 7);
    meta.set_field_a("updated");
    expect_mut_ref(meta.field_a_mut());
    meta.field_a_mut().push('!');
    assert_eq!(meta.field_a(), "updated!");
    assert!(meta.hidden);
}

#[test]
fn ui() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/accessors/pass/basic.rs");
    tests.pass("tests/ui/accessors/pass/fixed_api.rs");
    tests.compile_fail("tests/ui/accessors/fail/chained_setter.rs");
    tests.compile_fail("tests/ui/accessors/fail/enum.rs");
    tests.compile_fail("tests/ui/accessors/fail/legacy_field_attr.rs");
    tests.compile_fail("tests/ui/accessors/fail/reducer_option.rs");
    tests.compile_fail("tests/ui/accessors/fail/skip_on_struct.rs");
    tests.compile_fail("tests/ui/accessors/fail/skip_with_visibility.rs");
    tests.compile_fail("tests/ui/accessors/fail/skip_with_other_options.rs");
    tests.compile_fail("tests/ui/accessors/fail/tuple_struct.rs");
}
