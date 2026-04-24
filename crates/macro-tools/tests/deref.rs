use picea_macro_tools::Deref;

#[test]
fn runtime_generates_deref_for_a_single_target_field() {
    #[derive(Deref)]
    struct Wrapper {
        #[deref]
        value: String,
        label: &'static str,
    }

    let wrapper = Wrapper {
        value: String::from("picea"),
        label: "meta",
    };

    assert_eq!(wrapper.len(), 5);
    assert_eq!(&*wrapper, "picea");
    assert_eq!(wrapper.label, "meta");
}

#[test]
fn runtime_generates_deref_mut_only_when_opted_in() {
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
}

#[test]
fn runtime_supports_generics_and_where_clauses() {
    #[derive(Deref)]
    struct Wrapper<'a, T>
    where
        T: AsRef<str>,
    {
        #[deref]
        value: T,
        label: &'a str,
    }

    let wrapper = Wrapper {
        value: String::from("picea"),
        label: "meta",
    };

    assert_eq!(wrapper.as_str(), "picea");
    assert_eq!(wrapper.label, "meta");
}

#[test]
fn ui() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/deref/pass/basic.rs");
    tests.pass("tests/ui/deref/pass/generics.rs");
    tests.pass("tests/ui/deref/pass/mut.rs");
    tests.compile_fail("tests/ui/deref/fail/deref_mut_without_opt_in.rs");
    tests.compile_fail("tests/ui/deref/fail/duplicate_attribute.rs");
    tests.compile_fail("tests/ui/deref/fail/duplicate_mut.rs");
    tests.compile_fail("tests/ui/deref/fail/empty_args.rs");
    tests.compile_fail("tests/ui/deref/fail/enum.rs");
    tests.compile_fail("tests/ui/deref/fail/multi_target.rs");
    tests.compile_fail("tests/ui/deref/fail/name_value.rs");
    tests.compile_fail("tests/ui/deref/fail/no_target.rs");
    tests.compile_fail("tests/ui/deref/fail/tuple_struct.rs");
    tests.compile_fail("tests/ui/deref/fail/unsupported_option.rs");
    tests.compile_fail("tests/ui/deref/fail/unit_struct.rs");
}
