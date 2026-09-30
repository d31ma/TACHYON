use super::TypeScriptFieldType;

fn typed(text: &str) -> (bool, String) {
    let field = TypeScriptFieldType::new(text);
    (field.is_required(), field.type_text())
}

#[test]
fn model_field_text_becomes_a_typescript_type() {
    assert_eq!(typed("!string"), (true, String::from("string")));
    assert_eq!(typed("integer"), (false, String::from("number")));
    assert_eq!(typed("!any"), (true, String::from("unknown")));
    assert_eq!(typed("?Line"), (false, String::from("Line | null")));
    assert_eq!(
        typed("![]?boolean"),
        (true, String::from("(boolean | null)[]"))
    );
    assert_eq!(typed("[][]number"), (false, String::from("number[][]")));
}
