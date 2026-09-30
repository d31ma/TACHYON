use super::ElementDeclaration;

#[test]
fn an_event_name_is_the_model_name_in_kebab_case() {
    assert_eq!(
        ElementDeclaration::event_name("CountChanged"),
        "count-changed"
    );
    assert_eq!(ElementDeclaration::event_name("Step2Done"), "step2-done");
    assert_eq!(ElementDeclaration::event_name("HTTPReady"), "httpready");
}
