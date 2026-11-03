use super::validate;
use crate::behavior::RenamedAnnotation;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

struct Spelling;

impl Spelling {
    const LANGUAGES: [(&str, &str, &str, &str); 11] = [
        ("js", "@", "", "\nclass Plain {}\n"),
        ("ts", "@", "", "\nclass Plain {}\n"),
        ("py", "@", "", "\nclass Plain:\n    pass\n"),
        ("java", "@", "", "\nclass Plain {}\n"),
        ("kt", "@", "", "\nclass Plain\n"),
        ("dart", "@", "", "\nclass Plain {}\n"),
        ("swift", "@", "", "\nfinal class Plain {}\n"),
        ("cs", "[", "]", "\npublic class Plain {}\n"),
        ("php", "#[", "]", "\nclass Plain {}\n"),
        ("rs", "#[", "]", "\nstruct Plain;\n"),
        ("go", "// @", "", "\ntype Plain struct{}\n"),
    ];

    fn source(prefix: &str, suffix: &str, name: &str, declaration: &str) -> String {
        format!("{prefix}{name}{suffix}{declaration}")
    }
}

fn refusal(path: &str, source: &str) -> Diagnostic {
    let failure = validate(Path::new(path), source).expect_err(source);
    failure.diagnostics()[0].clone()
}

#[test]
fn every_old_annotation_name_is_refused_in_every_language() {
    for (extension, prefix, suffix, declaration) in Spelling::LANGUAGES {
        for (old, new) in RenamedAnnotation::NAMES {
            let path = format!("server/services/PlainService.{extension}");
            let source = Spelling::source(prefix, suffix, old, declaration);
            let diagnostic = refusal(&path, &source);
            assert_eq!(diagnostic.code.number(), 1308, "{source}");
            assert!(
                diagnostic
                    .message
                    .starts_with(&format!("'{old}' was renamed to {new}")),
                "{extension}: {}",
                diagnostic.message
            );
        }
    }
}

#[test]
fn snake_case_and_namespaced_spellings_are_refused_with_the_same_spelling() {
    let python = refusal(
        "server/services/PlainService.py",
        "@post_construct\nclass Plain:\n    pass\n",
    );
    assert!(
        python
            .message
            .contains("renamed to on_start; write on_start"),
        "{}",
        python.message
    );
    let rust = refusal(
        "server/services/PlainService.rs",
        "#[yon::cache_config({})]\nstruct Plain;\n",
    );
    assert!(
        rust.message.contains("renamed to cache_settings"),
        "{}",
        rust.message
    );
    let help = rust.help.unwrap_or_default();
    assert!(help.contains("ty migrate fix --write"), "{help}");
}

#[test]
fn an_old_name_inside_a_string_or_comment_is_not_refused() {
    let source = "@Service\nclass PlainService {\n  // @Autowired\n  label = '@Scope'\n}\n";
    validate(Path::new("server/services/PlainService.js"), source).expect("inert text");
}

#[test]
fn a_python_property_setter_named_like_an_old_annotation_is_not_refused() {
    let source = "@service\nclass PlainService:\n    @property\n    def value(self):\n        return 1\n\n    @value.setter\n    def value(self, item):\n        pass\n";
    validate(Path::new("server/services/PlainService.py"), source).expect("setter");
}

#[test]
fn a_file_in_an_old_role_folder_is_refused_with_the_new_folder() {
    for (old, new) in super::RenamedFolder::NAMES {
        for realm in ["client", "server"] {
            let path = format!("{realm}/{old}/PlainStore.js");
            let failure = super::reject_reserved_folder(Path::new(&path)).expect_err(&path);
            let diagnostic = &failure.diagnostics()[0];
            assert_eq!(diagnostic.code.number(), 1310, "{path}");
            assert!(
                diagnostic
                    .message
                    .contains(&format!("renamed to {realm}/{new}")),
                "{}",
                diagnostic.message
            );
        }
    }
    super::reject_reserved_folder(Path::new(
        "server/controllers/repositories/MoneyController.js",
    ))
    .expect("only a realm's own role folder is renamed");
}

#[test]
fn the_rust_test_attribute_is_not_the_test_marker() {
    let source = "#[Service]\npub struct MoneyService;\n\nimpl MoneyService {\n    #[test]\n    fn adds() {}\n}\n";
    validate(Path::new("server/services/MoneyService.rs"), source).expect("rust test attribute");
    assert!(
        !super::has_test_annotation(Path::new("server/services/MoneyService.rs"), source)
            .expect("scan")
    );
}

#[test]
fn the_component_role_is_refused_with_the_delegate_role() {
    let marker = refusal(
        "server/services/PlainService.go",
        "// @Component\ntype Plain struct{}\n",
    );
    assert!(
        marker
            .message
            .starts_with("'Component' was renamed to Delegate"),
        "{}",
        marker.message
    );
    let python = refusal(
        "server/services/PlainService.py",
        "@component\nclass Plain:\n    pass\n",
    );
    assert!(
        python
            .message
            .contains("renamed to delegate; write delegate"),
        "{}",
        python.message
    );
    super::reject_reserved_folder(Path::new("client/delegates/PriceDelegate.ts"))
        .expect("delegates is the Delegate role folder again (ADR 0112)");
    validate(
        Path::new("client/delegates/PriceDelegate.ts"),
        "@Delegate\nclass PriceDelegate {}\n",
    )
    .expect("the new delegate role");
}

#[test]
fn the_helper_role_is_refused_with_the_delegate_role() {
    let marker = refusal(
        "server/services/PlainService.go",
        "// @Helper\ntype Plain struct{}\n",
    );
    assert!(
        marker
            .message
            .starts_with("'Helper' was renamed to Delegate"),
        "{}",
        marker.message
    );
    let python = refusal(
        "server/services/PlainService.py",
        "@helper\nclass Plain:\n    pass\n",
    );
    assert!(
        python
            .message
            .contains("renamed to delegate; write delegate"),
        "{}",
        python.message
    );
}
