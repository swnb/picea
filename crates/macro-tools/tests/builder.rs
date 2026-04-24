use picea_macro_tools::Builder;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn runtime_builds_required_default_and_skipped_fields() {
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

    assert_eq!(
        settings,
        Settings {
            name: String::from("picea"),
            retries: 0,
            profile: String::from("stable"),
            cached: Vec::new(),
        }
    );
}

#[test]
fn runtime_returns_error_when_required_field_is_missing() {
    #[derive(Debug, PartialEq, Builder)]
    struct Settings {
        name: String,
        #[builder(default)]
        retries: usize,
    }

    let error = SettingsBuilder::new().build().unwrap_err();

    assert_eq!(error, "missing field: name");
}

#[test]
fn runtime_supports_generics_and_where_clauses() {
    #[derive(Debug, PartialEq, Builder)]
    struct Envelope<T>
    where
        T: Clone + PartialEq + core::fmt::Debug,
    {
        payload: T,
        #[builder(default)]
        tags: Vec<String>,
        #[builder(skip, default = 7)]
        priority: usize,
    }

    let envelope = EnvelopeBuilder::<String>::new()
        .payload("payload")
        .tags(vec![String::from("macro")])
        .build()
        .unwrap();

    assert_eq!(
        envelope,
        Envelope {
            payload: String::from("payload"),
            tags: vec![String::from("macro")],
            priority: 7,
        }
    );
}

#[test]
fn runtime_evaluates_default_values_only_during_build() {
    static DEFAULT_CALLS: AtomicUsize = AtomicUsize::new(0);
    static EXPR_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SKIP_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, PartialEq)]
    struct CounterDefault;

    impl Default for CounterDefault {
        fn default() -> Self {
            DEFAULT_CALLS.fetch_add(1, Ordering::SeqCst);
            Self
        }
    }

    fn explicit_default() -> String {
        EXPR_CALLS.fetch_add(1, Ordering::SeqCst);
        String::from("from-expr")
    }

    fn skipped_default() -> Vec<String> {
        SKIP_CALLS.fetch_add(1, Ordering::SeqCst);
        vec![String::from("skip")]
    }

    #[derive(Debug, PartialEq, Builder)]
    struct Settings {
        name: String,
        #[builder(default)]
        counter: CounterDefault,
        #[builder(default = explicit_default())]
        profile: String,
        #[builder(skip, default = skipped_default())]
        cache: Vec<String>,
    }

    DEFAULT_CALLS.store(0, Ordering::SeqCst);
    EXPR_CALLS.store(0, Ordering::SeqCst);
    SKIP_CALLS.store(0, Ordering::SeqCst);

    let builder = SettingsBuilder::new();

    assert_eq!(DEFAULT_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(EXPR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(SKIP_CALLS.load(Ordering::SeqCst), 0);

    let settings = builder.name("picea").build().unwrap();

    assert_eq!(DEFAULT_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(EXPR_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(SKIP_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(settings.profile, "from-expr");
    assert_eq!(settings.cache, vec![String::from("skip")]);
}

#[test]
fn runtime_skips_default_expressions_when_the_field_is_set() {
    static DEFAULT_CALLS: AtomicUsize = AtomicUsize::new(0);
    static EXPR_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, PartialEq)]
    struct CounterDefault;

    impl Default for CounterDefault {
        fn default() -> Self {
            DEFAULT_CALLS.fetch_add(1, Ordering::SeqCst);
            Self
        }
    }

    fn explicit_default() -> String {
        EXPR_CALLS.fetch_add(1, Ordering::SeqCst);
        String::from("from-expr")
    }

    #[derive(Debug, PartialEq, Builder)]
    struct Settings {
        name: String,
        #[builder(default)]
        counter: CounterDefault,
        #[builder(default = explicit_default())]
        profile: String,
    }

    DEFAULT_CALLS.store(0, Ordering::SeqCst);
    EXPR_CALLS.store(0, Ordering::SeqCst);

    let settings = SettingsBuilder::new()
        .name("picea")
        .counter(CounterDefault)
        .profile("custom")
        .build()
        .unwrap();

    assert_eq!(DEFAULT_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(EXPR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(settings.profile, "custom");
}

#[test]
fn ui() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/builder/pass/basic.rs");
    tests.pass("tests/ui/builder/pass/generics.rs");
    tests.compile_fail("tests/ui/builder/fail/duplicate_default.rs");
    tests.compile_fail("tests/ui/builder/fail/duplicate_skip.rs");
    tests.compile_fail("tests/ui/builder/fail/origin_default.rs");
    tests.compile_fail("tests/ui/builder/fail/reserved_build.rs");
    tests.compile_fail("tests/ui/builder/fail/reserved_new.rs");
    tests.compile_fail("tests/ui/builder/fail/skip_without_default.rs");
    tests.compile_fail("tests/ui/builder/fail/struct_level_attribute.rs");
    tests.compile_fail("tests/ui/builder/fail/tuple_struct.rs");
    tests.compile_fail("tests/ui/builder/fail/unsupported_option.rs");
}
