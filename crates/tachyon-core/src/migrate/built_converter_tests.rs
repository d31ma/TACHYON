use super::built_converter::BuiltConverter;

#[test]
fn a_converter_built_with_a_context_is_reported_in_every_form() {
    let built = [
        (
            "A.java",
            "ProductConverter mapper = new ProductConverter(context);",
        ),
        ("A.kt", "val mapper = ProductConverter(context)"),
        (
            "A.rs",
            "let mapper = ProductConverter::new(context.clone());",
        ),
        (
            "A.js",
            "const mapper = Behaviors.construct(ProductConverter, { behaviors: context });",
        ),
        (
            "A.php",
            "$mapper = Behaviors::construct(ProductConverter::class, ['behaviors' => $context]);",
        ),
        ("A.go", "mapper := &ProductConverter{behaviors: context}"),
        ("A.rb", "mapper = ProductConverter.new(context)"),
    ];
    for (path, source) in built {
        let finding = BuiltConverter::finding(path, source);
        assert!(finding.is_some(), "{path}: {source}");
    }
}

#[test]
fn an_injected_or_declared_converter_is_not_reported() {
    let kept = [
        ("A.java", "@Needs\nProductConverter converter;"),
        ("A.py", "class ProductConverter(Protocol):\n    pass\n"),
        (
            "A.fs",
            "type CatalogController(converter: ProductConverter) =",
        ),
        (
            "A.swift",
            "private let converter: ProductConverter = ProductConverter()",
        ),
        ("A.java", "// new ProductConverter(context)"),
        ("A.java", "@Converter\ninterface ProductConverter {}"),
        ("A.kt", "val converter = makeConverter(settings)"),
    ];
    for (path, source) in kept {
        assert!(
            BuiltConverter::finding(path, source).is_none(),
            "{path}: {source}"
        );
    }
}
