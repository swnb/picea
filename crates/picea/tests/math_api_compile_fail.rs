#[test]
fn legacy_math_api_is_not_available() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/math_legacy_api/*.rs");
}
