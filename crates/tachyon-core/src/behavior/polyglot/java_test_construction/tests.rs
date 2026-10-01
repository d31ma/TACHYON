use std::path::Path;

fn lowered(fake: &str) -> String {
    let source = format!(
        "@Test(TestSetup)\nclass ChoiceServiceTest {{\n    {fake}\n    void testRead() {{}}\n}}\n\
         @TestSetup\nclass TestSetup {{\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {{\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null);\n    }}\n}}\n\
         class ChoiceService {{\n    @Needs\n    String value;\n}}\n"
    );
    crate::behavior::lower(
        Path::new("client/tests/ChoiceServiceTest.java"),
        &source,
        "tac",
    )
    .expect("Test with declared fake")
    .expect("Java lowering")
}

#[test]
fn declared_fake_values_are_passed_to_the_real_typed_constructor() {
    let source = lowered("@Fake\n    String value;");
    assert!(source.contains("public ChoiceService(String value)"));
    assert!(
        source.contains("type.cast(new ChoiceService(java.util.Objects.requireNonNull(this.value")
    );
    assert!(source.contains("final String value;"));
    assert!(!source.contains("java.lang.reflect"));
    assert!(!source.contains("getDeclaredField"));
}

#[test]
fn an_unmarked_field_does_not_become_a_fake() {
    let source = lowered("String value;");
    assert!(source.contains("needs a matching typed Fake named value for ChoiceService"));
    assert!(!source.contains("new ChoiceService("));
}

#[test]
fn an_incompatible_fake_is_refused_before_emitting_an_invalid_constructor_call() {
    let source = lowered("@Fake\n    long value;");
    assert!(source.contains("needs a matching typed Fake named value for ChoiceService"));
    assert!(!source.contains("new ChoiceService("));
}

#[test]
fn construction_requires_active_test_setup_and_an_assigned_fake() {
    let source = lowered("@Fake\n    String value;");
    assert!(source.contains("this.behaviors == null"));
    assert!(source.contains("Test.construct needs an assigned Fake named value"));
    assert!(source.contains("Test.construct needs a class with a declared Needs constructor"));
    assert!(!source.contains("static <T> T construct"));
}
