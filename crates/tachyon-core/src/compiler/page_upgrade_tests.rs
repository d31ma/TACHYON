use super::PageUpgrade;
use std::collections::BTreeSet;

#[test]
fn a_page_stylesheet_splits_rule_by_rule_and_keeps_its_text() {
    let source = "/* page */\nbody { margin: 0; color: #e2e8f0; }\n.shell { max-width: 72rem; }\n:root { color-scheme: dark; }\n.brand a { color: inherit; }\n";
    let split = PageUpgrade::split_styles(source).expect("split");
    assert_eq!(
        split.foundation,
        "body { margin: 0; color: #e2e8f0; }\n:root { color-scheme: dark; }\n"
    );
    assert_eq!(
        split.component,
        "/* page */\n.shell { max-width: 72rem; }\n.brand a { color: inherit; }\n"
    );
    assert!(PageUpgrade::split_styles("body { color: red;").is_none());
}

#[test]
fn page_markup_diagnostics_are_the_ones_bundle_gives() {
    let names = BTreeSet::from([String::from("hero")]);
    let refused =
        PageUpgrade::markup_diagnostics("<main><hero /></main>", "client/pages/tac.html", &names);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(refused[0].code.number(), 1309);
    assert!(
        PageUpgrade::markup_diagnostics("<hero />\n", "client/pages/tac.html", &names).is_empty()
    );
}
