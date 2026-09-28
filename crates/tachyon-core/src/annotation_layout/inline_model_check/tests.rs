use super::InlineModelCheck;
use std::path::Path;

#[test]
fn a_javascript_typedef_outside_the_models_folder_is_refused() {
    let source = "/**\n * @typedef {Object} CartTotal\n * @property {number} total\n */\n@Service\nexport default class CartService {}\n";
    let failure =
        InlineModelCheck::refuse_inline_models(Path::new("client/services/CartService.js"), source)
            .expect_err("inline model");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 1310);
    assert!(
        diagnostic
            .message
            .contains("declares the model 'CartTotal' with a @typedef"),
        "{}",
        diagnostic.message
    );
    let help = diagnostic.help.as_deref().unwrap_or_default();
    assert!(help.contains("'client/models/CartTotal.js'"), "{help}");
    assert!(help.contains("ty migrate fix --write"), "{help}");
}

#[test]
fn model_files_other_languages_and_files_outside_a_realm_are_not_checked() {
    let source = "/** @typedef {Object} CartTotal */\nexport {}\n";
    for path in [
        "client/models/CartTotal.js",
        "server/models/orders/CartTotal.js",
        "client/services/CartService.ts",
        "middleware.js",
        "scripts/build.js",
    ] {
        assert!(
            InlineModelCheck::refuse_inline_models(Path::new(path), source).is_ok(),
            "{path}"
        );
    }
    let plain = "@Service\nexport default class CartService {}\n";
    assert!(
        InlineModelCheck::refuse_inline_models(Path::new("server/services/CartService.js"), plain)
            .is_ok()
    );
}

#[test]
fn the_rule_stays_off_until_every_project_passes_it() {
    let source = "/** @typedef {Object} CartTotal */\n";
    let result = InlineModelCheck::check(Path::new("client/services/CartService.js"), source);
    assert_eq!(result.is_ok(), !InlineModelCheck::ENFORCED);
}
