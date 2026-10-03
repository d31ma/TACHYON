#![allow(clippy::expect_used)]

use super::SharedImports;
use crate::annotation_modules::merge_imports;
use std::path::Path;

#[test]
fn shared_imports_canonicalizes_only_allowed_captured_relative_paths() {
    let entry = Path::new("client/services/LabelService.js");
    for specifier in [
        "../shared/utils/nested/TitleUtils.js",
        "././../shared/utils/nested/TitleUtils.js",
        "../shared/utils/nested/../nested/TitleUtils.js",
    ] {
        assert_eq!(
            SharedImports::canonical(entry, specifier),
            "../shared/utils/nested/TitleUtils.js"
        );
    }
    for specifier in [
        "../../server/shared/TitleUtils.js",
        "../../../client/shared/utils/TitleUtils.js",
        "../services/OtherService.js",
        "external-package",
        "/shared/utils/TitleUtils.js",
        "../utils\\TitleUtils.js",
    ] {
        assert_eq!(SharedImports::canonical(entry, specifier), specifier);
    }
    assert_eq!(
        SharedImports::canonical(
            Path::new("client/shared/utils/LabelUtils.js"),
            "./nested/../TitleUtils.js"
        ),
        "./TitleUtils.js"
    );
}

#[test]
fn shared_imports_merges_equivalent_paths_without_erasing_real_module_identity() {
    let path = Path::new("client/services/LabelService.js");
    let sources = [
        "import { TitleUtils } from '../shared/utils/nested/TitleUtils.js';\nclass LabelService {}",
        "import { TitleUtils } from '././../shared/utils/nested/TitleUtils.js';\nclass UnusedService {}",
    ];
    let output = merge_imports(path, sources.into_iter().map(str::to_owned).collect())
        .expect("equivalent captured utility imports");
    assert_eq!(
        output
            .matches("from \"../shared/utils/nested/TitleUtils.js\"")
            .count(),
        1
    );
    for specifier in [
        "../shared/utils/other/TitleUtils.js",
        "../shared/TitleUtils.js",
        "../shared/utils/nested/TitleUtils.js?v=1",
        "../shared/utils/nested/TitleUtils.js#loaded",
    ] {
        let second =
            format!("import {{ TitleUtils }} from '{specifier}';\nclass UnusedService {{}}");
        assert!(
            merge_imports(path, vec![sources[0].to_owned(), second]).is_err(),
            "{specifier}"
        );
    }
    assert_eq!(
        SharedImports::canonical(path, "././../shared/utils/nested/TitleUtils.js?v=1#loaded"),
        "../shared/utils/nested/TitleUtils.js?v=1#loaded"
    );
}

#[test]
fn shared_imports_merge_uses_entry_base_after_nested_role_rebasing() {
    let entry = Path::new("client/services/LabelService.js");
    let role = Path::new("client/helpers/nested/LabelHelper.js");
    let nested = SharedImports::rebase(
        entry,
        role,
        "import { TitleUtils } from '../../shared/utils/nested/TitleUtils.js';\nclass LabelHelper {}",
        "js",
    )
    .expect("captured nested role import");
    let authored = "import { TitleUtils } from '././../shared/utils/nested/TitleUtils.js';\nclass LabelService {}";
    let output = merge_imports(entry, vec![nested, authored.to_owned()])
        .expect("entry and already rebased module use the same base");
    assert_eq!(
        output
            .matches("from \"../shared/utils/nested/TitleUtils.js\"")
            .count(),
        1
    );
}
