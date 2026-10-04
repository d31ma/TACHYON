use super::DesignCheck;
use super::DesignSwitches;
use super::project_words::ProjectWords;
use std::path::Path;

fn messages(code: u16, files: &[(&str, &str)]) -> Vec<String> {
    let pairs = files
        .iter()
        .map(|(path, text)| (Path::new(*path), text.as_bytes()));
    DesignCheck::diagnostics(pairs, &DesignSwitches::every_realm(ProjectWords::default()))
        .into_iter()
        .filter(|problem| problem.code.number() == code)
        .map(|problem| problem.message)
        .collect()
}

#[test]
fn a_utility_that_names_a_role_class_is_refused_in_both_realms() {
    let service = "@Service\nexport default class PriceService {\n  total: number = 0\n}\n";
    let utility = "export default class PriceUtils {\n  static total(prices: PriceService): number {\n    return prices.total\n  }\n}\n";
    for (service_path, utility_path) in [
        (
            "client/services/PriceService.ts",
            "client/shared/utils/money/PriceUtils.ts",
        ),
        (
            "server/services/PriceService.ts",
            "server/utils/PriceUtils.ts",
        ),
    ] {
        let found = messages(1327, &[(service_path, service), (utility_path, utility)]);
        assert_eq!(
            found,
            [
                "PriceUtils is a utility, and it names PriceService, which is a service. A utility depends on no role class."
            ],
            "{utility_path}"
        );
    }
}

#[test]
fn go_and_rust_utilities_that_name_a_store_are_refused() {
    let go_store = "// @Store\ntype OrderStore struct{}\n";
    let go_utility = "type OrderUtils struct{}\n\nfunc (OrderUtils) Count(store *OrderStore) int {\n\treturn 0\n}\n";
    let found = messages(
        1327,
        &[
            ("server/stores/OrderStore.go", go_store),
            ("server/utils/OrderUtils.go", go_utility),
        ],
    );
    assert_eq!(found.len(), 1, "{found:?}");
    let rust_store = "#[Store]\nstruct OrderStore {}\n";
    let rust_utility = "pub struct OrderUtils;\n\nimpl OrderUtils {\n    pub fn count(store: &OrderStore) -> usize {\n        0\n    }\n}\n";
    let rust = messages(
        1327,
        &[
            ("server/stores/OrderStore.rs", rust_store),
            ("server/utils/order_utils.rs", rust_utility),
        ],
    );
    assert_eq!(rust.len(), 1, "{rust:?}");
}

#[test]
fn utilities_may_use_other_utilities_and_every_role_may_use_a_utility() {
    let text = "export default class TextUtils {\n  static trim(value) {\n    return String(value).trim()\n  }\n}\n";
    let label = "export default class LabelUtils {\n  static label(value) {\n    return TextUtils.trim(value)\n  }\n}\n";
    let service =
        "@Service\nexport default class LabelService {\n  label = LabelUtils.label(' x ')\n}\n";
    let found = messages(
        1327,
        &[
            ("client/shared/utils/TextUtils.js", text),
            ("client/shared/utils/LabelUtils.js", label),
            ("client/services/LabelService.js", service),
        ],
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn client_shared_utilities_keep_only_the_dependency_rule_of_the_design_rules() {
    let utility = "export default class TxtUtils {\n  static trim(value) {\n    return String(value).trim()\n  }\n}\n";
    let server = messages(1328, &[("server/utils/TxtUtils.js", utility)]);
    assert!(
        server
            .iter()
            .any(|message| message.contains("`txt` is not an English word")),
        "{server:?}"
    );
    let client = messages(1328, &[("client/shared/utils/TxtUtils.js", utility)]);
    assert!(client.is_empty(), "{client:?}");
}
