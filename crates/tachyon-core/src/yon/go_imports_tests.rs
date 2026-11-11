use super::GoImports;

const GROUPED: &str = "//tachyon:file OrdersController server/controllers/OrdersController.go\nimport (\n\t\"fmt\"\n\tshop \"example.com/shop/packages/core\"\n)\n\ntype OrdersController struct{}\n";

#[test]
fn a_package_main_type_needs_no_import() {
    assert_eq!(
        GoImports::for_types(GROUPED, &["Order", "any"]),
        Ok(String::new())
    );
}

#[test]
fn a_grouped_alias_import_is_copied_for_the_type() {
    assert_eq!(
        GoImports::for_types(GROUPED, &["*shop.OrderMissing"]),
        Ok(String::from(
            "import shop \"example.com/shop/packages/core\"\n"
        ))
    );
}

#[test]
fn a_single_import_is_named_after_its_last_path_part() {
    let source = "import \"example.com/shop/packages/core\"\n";
    assert_eq!(
        GoImports::for_types(source, &["[]core.Order"]),
        Ok(String::from(
            "import core \"example.com/shop/packages/core\"\n"
        ))
    );
}

#[test]
fn every_package_in_a_map_type_is_imported_once() {
    let source = "import \"time\"\nimport \"example.com/shop/packages/core\"\n";
    assert_eq!(
        GoImports::for_types(
            source,
            &["map[string]core.Order", "core.Order", "time.Time"]
        ),
        Ok(String::from(
            "import core \"example.com/shop/packages/core\"\nimport time \"time\"\n"
        ))
    );
}

#[test]
fn a_type_from_a_package_that_no_file_imports_is_refused() {
    let refusal = GoImports::for_types(GROUPED, &["core.Order"]).expect_err("missing import");
    assert!(refusal.contains("'core'"), "{refusal}");
}

#[test]
fn two_files_that_import_different_packages_under_one_name_are_refused() {
    let source = "import core \"example.com/a/core\"\n//tachyon:file Other server/services/Other.go\nimport core \"example.com/b/core\"\n";
    let refusal = GoImports::for_types(source, &["core.Order"]).expect_err("ambiguous import");
    assert!(
        refusal.contains("example.com/a/core, example.com/b/core"),
        "{refusal}"
    );
}
