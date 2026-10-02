#![allow(clippy::expect_used)]

use std::path::Path;

fn lowered(fake: &str) -> String {
    let source = format!(
        "@Test(ChoiceTestSetup)\nclass ChoiceServiceTest {{\n {fake}\n Future<void> testChoice() async {{}}\n}}\n\
         @TestSetup\nclass ChoiceTestSetup {{\n static BehaviorAdapters configure(BehaviorExecution execution) {{ return BehaviorAdapters(); }}\n}}\n\
         @Service\nclass ChoiceService {{\n @Needs\n final String value;\n String answer() {{ return value; }}\n}}\n"
    );
    let path = Path::new("client/tests/ChoiceServiceTest.dart");
    let source = crate::behavior::ServiceRoles::read(path, &source)
        .expect("roles")
        .source;
    crate::behavior::lower(path, &source, "tac")
        .expect("Test lowering")
        .expect("source")
}

#[test]
fn matching_nullable_fake_uses_the_real_final_constructor() {
    let source = lowered("@Fake\n String? value;");
    assert!(source.contains("return ChoiceService(_tachyonAssignedFake(this.value,"));
    assert!(source.contains("final String value;"));
    assert!(source.contains("ChoiceService(this.value);"));
    assert!(!source.contains("instance.value ="));
    assert!(!source.contains("static T construct"));
}

#[test]
fn missing_and_incompatible_fake_fields_refuse_construction() {
    for field in ["String? value;", "@Fake\n int? value;"] {
        let source = lowered(field);
        assert!(source.contains("matching typed Fake named value for ChoiceService"));
        assert!(!source.contains("return ChoiceService(_tachyonAssignedFake("));
    }
}

#[test]
fn construction_requires_active_setup_and_assigned_value() {
    let source = lowered("@Fake\n String? value;");
    assert!(source.contains("if (behaviors == null) throw StateError("));
    assert!(source.contains("Test.construct needs an assigned Fake named value."));
    assert!(source.contains("if (value == null) throw StateError(message)"));
    assert!(source.contains("Test.construct needs a class with a declared Needs constructor."));
}

#[test]
fn a_type_argument_mismatch_is_refused_before_a_constructor_call() {
    let source = lowered("@Fake\n String? value;");
    let guard = source
        .find("if (type != T) throw ArgumentError(")
        .expect("type guard");
    let factory = source
        .find("return ChoiceService(")
        .expect("typed constructor");
    assert!(guard < factory);
    assert!(source.contains("Test.construct needs a target matching its type argument."));
}
