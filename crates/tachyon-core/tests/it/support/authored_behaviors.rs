use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

pub fn write(root: &Path, language: &str, realm: &str, extension: &str) {
    let helpers = role_names(language);
    for (relative, source) in files(language) {
        let source = super::behavior_layout::rename(source, &helpers);
        let (relative, mut source) = role_copy(relative, &source, &helpers, language);
        let relative = model_copy(relative, language);
        let (relative, copied) = csharp_data_copy(relative, source, language);
        source = copied;
        source = model_source(&relative, source, language);
        source = ruby_converter_source(&relative, source, language);
        source = php_helper_source(&relative, source, language);
        source = csharp_helper_source(&relative, source, language);
        let (relative, mut source) = tac_checks_copy(relative, source, realm, language);
        let relative = relative.replacen("server/", &format!("{realm}/"), 1);
        let relative = Path::new(&relative).with_extension(extension);
        if language == "js" && realm == "server" {
            source = source
                .replace("TAC_", "YON_")
                .replace("tac.", "yon.")
                .replace("tac: {", "yon: {");
        }
        let target = root.join(relative);
        std::fs::create_dir_all(target.parent().expect("fixture parent"))
            .expect("fixture directories");
        std::fs::write(target, source).expect("authored fixture");
    }
}

fn tac_checks_copy(
    relative: String,
    source: String,
    realm: &str,
    language: &str,
) -> (String, String) {
    if realm != "client" || language != "js" {
        return (relative, source);
    }
    let names = BTreeMap::from([(
        "BehaviorChecksHelper".to_owned(),
        "BehaviorChecks".to_owned(),
    )]);
    let source = super::behavior_layout::rename(&source, &names)
        .replace(
            "@Helper()\nclass BehaviorChecks {",
            "class BehaviorChecks {",
        )
        .replace(
            "\nclass InputService {",
            "\nexport default class InputService {",
        );
    let relative = relative.replacen(
        "server/helpers/BehaviorChecksHelper.js",
        "server/tests/BehaviorChecks.js",
        1,
    );
    (relative, source)
}

fn csharp_helper_source(relative: &str, source: String, language: &str) -> String {
    if language != "cs" {
        return source;
    }
    match relative {
        "server/helpers/GuardedConfigHelper.cs" | "server/helpers/MissingConfigHelper.cs" => source
            .replace(
                "public static BehaviorAdapters configure(",
                "private static BehaviorAdapters configure(",
            ),
        "server/helpers/CatalogExercisesHelper.cs" => {
            let at = source
                .find("        var catalog = new AnnotatedCatalogService")
                .expect("existing owned behavior context precedes catalog construction");
            let mut copied = source;
            copied.insert_str(at, r#"        var resilient = new AnnotatedResilientHelper(context);
        var attempts = await resilient.flaky();
        if (attempts.Integer != 3 || resilient.attempts != 3) throw new System.Exception("owned helper retry failed");
        var bounded = await resilient.bounded(BehaviorJSON.From("ok"));
        if (bounded.Text != "ok") throw new System.Exception("owned helper concurrency bound failed");
"#);
            copied
        }
        _ => source,
    }
}

fn php_helper_source(relative: &str, mut source: String, language: &str) -> String {
    if language != "php" || relative != "server/helpers/HandlerChecksHelper.php" {
        return source;
    }
    for method in [
        "behaviorError",
        "exerciseCache",
        "checkResilience",
        "checkQueries",
        "checkMapping",
        "isCancelled",
    ] {
        source = source.replace(
            &format!("public static function {method}("),
            &format!("private static function {method}("),
        );
    }
    source
}

fn ruby_converter_source(relative: &str, source: String, language: &str) -> String {
    if language != "rb" {
        return source;
    }
    match relative {
        "server/converters/ProductSummariesConverter.rb" => {
            source.replace("  attr_accessor :behaviors\n", "")
        }
        "server/controllers/ProductsController.rb" => source
            .replace("class ProductsController\n", "class ProductsController\n  #[Needs]\n  # @return [ProductSummariesConverter]\n  attr_reader :product_summaries_converter\n")
            .replace("HandlerChecksHelper.exercise(context, signal)", "HandlerChecksHelper.exercise(context, signal, product_summaries_converter)"),
        "server/helpers/HandlerChecksHelper.rb" => source
            .replace("def self.check_mapping(context)\n    summaries = Behaviors.construct(ProductSummariesConverter, behaviors: context)", "def self.check_mapping(summaries)")
            .replace("def self.exercise(context, signal)", "def self.exercise(context, signal, summaries)")
            .replace("check_mapping(context)", "check_mapping(summaries)"),
        _ => source,
    }
}

fn model_copy(relative: String, language: &str) -> String {
    let name = Path::new(&relative)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if metadata_names(language).contains(&name) {
        relative.replacen("server/helpers/", "server/models/", 1)
    } else {
        relative
    }
}

fn csharp_data_copy(relative: String, source: String, language: &str) -> (String, String) {
    let name = Path::new(&relative)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if language != "cs"
        || !["ApplicationProperties", "InvalidProperties", "Product"].contains(&name)
    {
        return (relative, source);
    }
    let marker = format!("[Description(\"Behavior conformance {name}\")]\n");
    let copied = source
        .strip_prefix(&marker)
        .expect("audited inert C# conformance description")
        .to_owned();
    let relative = if name == "ApplicationProperties" {
        relative.replacen("server/helpers/", "server/shared/", 1)
    } else {
        relative
    };
    (relative, copied)
}

fn metadata_names(language: &str) -> &'static [&'static str] {
    match language {
        "js" | "ts" => &["ApplicationSettings", "ApplicationValues", "ProductInput"],
        "rs" | "go" | "py" | "php" | "rb" => &["ApplicationValues", "Settings", "ProductInput"],
        "java" | "groovy" | "kt" => &["ApplicationValues", "Settings", "Input"],
        "cs" => &[
            "AnnotatedValues",
            "AnnotatedSettings",
            "AnnotatedInput",
            "AnnotatedPageInput",
            "InvalidProperties",
            "Product",
        ],
        _ => &[],
    }
}

fn model_source(relative: &str, source: String, language: &str) -> String {
    if relative.starts_with("server/models/") {
        return public_rust_model(relative, source, language);
    }
    let names = referenced_models(&source, language);
    model_imports(source, language, &names)
}

fn public_rust_model(relative: &str, source: String, language: &str) -> String {
    if language != "rs" {
        return source;
    }
    let name = Path::new(relative)
        .file_stem()
        .and_then(|name| name.to_str())
        .expect("audited model name");
    source.replacen(&format!("struct {name}"), &format!("pub struct {name}"), 1)
}

fn referenced_models(source: &str, language: &str) -> Vec<&'static str> {
    metadata_names(language)
        .iter()
        .copied()
        .filter(|name| {
            source
                .split(|value: char| !value.is_alphanumeric() && value != '_')
                .any(|word| word == *name)
        })
        .collect()
}

fn model_imports(source: String, language: &str, names: &[&str]) -> String {
    if names.is_empty() {
        return source;
    }
    let imports = match language {
        "rs" => model_import_lines(names, "use crate::models::", ";"),
        "kt" => model_import_lines(names, "import models.", ""),
        "cs" => "using Models;\n".to_owned(),
        "py" => format!("from models import {}\n", names.join(", ")),
        "php" => model_import_lines(names, "use Models\\", ";"),
        _ => String::new(),
    };
    if language == "php" {
        return source.replacen("<?php\n", &format!("<?php\n{imports}"), 1);
    }
    format!("{imports}{source}")
}

fn model_import_lines(names: &[&str], prefix: &str, suffix: &str) -> String {
    let mut imports = String::new();
    for name in names {
        writeln!(imports, "{prefix}{name}{suffix}").expect("write fixture imports to a String");
    }
    imports
}

fn role_names(language: &str) -> BTreeMap<String, String> {
    let names: &[&str] = match language {
        "rs" | "go" => &[
            "CatalogChecks",
            "Ledger",
            "Memory",
            "PureValidator",
            "Resilient",
        ],
        "groovy" | "kt" | "java" => &["Resilient", "Validator"],
        "cs" => &[
            "AnnotatedResilient",
            "CatalogExercises",
            "Config",
            "ConfigurationAnnotationTests",
            "GuardedConfig",
            "MissingConfig",
            "ValidationAnnotationTests",
        ],
        "py" | "php" | "rb" => &["Backend", "HandlerChecks", "Resilient"],
        _ => &[],
    };
    let mut roles: BTreeMap<_, _> = names
        .iter()
        .map(|name| ((*name).to_owned(), format!("{name}Helper")))
        .collect();
    if ["py", "php", "rb"].contains(&language) {
        roles.insert(
            "ProductSummaries".into(),
            "ProductSummariesConverter".into(),
        );
    }
    roles
}

fn role_copy(
    relative: &str,
    source: &str,
    names: &BTreeMap<String, String>,
    language: &str,
) -> (String, String) {
    let path = Path::new(relative);
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let role = names
        .get(stem)
        .map(|name| {
            if name.ends_with("Converter") {
                ("converters", "Converter", name.as_str())
            } else {
                ("helpers", "Helper", name.as_str())
            }
        })
        .or_else(|| service_copy(language, stem).map(|name| ("services", "Service", name)));
    role.map_or_else(
        || (relative.to_owned(), source.to_owned()),
        |(folder, marker, name)| {
            let source = marked_copy(source.to_owned(), language, name, marker);
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            (
                path.with_file_name(format!("{name}.{extension}"))
                    .to_string_lossy()
                    .replacen("server/helpers/", &format!("server/{folder}/"), 1),
                source,
            )
        },
    )
}

fn service_copy<'a>(language: &str, name: &'a str) -> Option<&'a str> {
    match (language, name) {
        ("js" | "ts", "InputService") => Some(name),
        ("cs", "AnnotatedValidationService") | ("py" | "php" | "rb", "ValidationService") => {
            Some(name)
        }
        _ => None,
    }
}

fn marked_copy(mut source: String, language: &str, name: &str, role: &str) -> String {
    let (declaration, marker) = match language {
        "rs" => ("struct", format!("#[{role}]")),
        "go" => ("type", format!("// @{role}")),
        "php" if source.contains(&format!("interface {name}")) => {
            ("interface", format!("#[{role}]"))
        }
        "php" | "rb" => ("class", format!("#[{role}]")),
        "cs" => ("class", format!("[{role}]")),
        "py" => ("class", format!("@{}", role.to_ascii_lowercase())),
        _ => ("class", format!("@{role}")),
    };
    let at = source
        .find(&format!("{declaration} {name}"))
        .expect("explicit fixture role declaration");
    let line = source[..at].rfind('\n').map_or(0, |at| at + 1);
    source.insert_str(line, &format!("{marker}\n"));
    source
}

fn files(language: &str) -> &'static [(&'static str, &'static str)] {
    match language {
        "js" => AuthoredFixtures::JS,
        "py" => AuthoredFixtures::PY,
        "php" => AuthoredFixtures::PHP,
        "rb" => AuthoredFixtures::RB,
        "java" => AuthoredFixtures::JAVA,
        "groovy" => AuthoredFixtures::GROOVY,
        "kt" => AuthoredFixtures::KT,
        "rs" => AuthoredFixtures::RS,
        "cs" => AuthoredFixtures::CS,
        "go" => AuthoredFixtures::GO,
        _ => panic!("unknown behavior fixture language: {language}"),
    }
}

struct AuthoredFixtures;

impl AuthoredFixtures {
    const JS: &[(&str, &str)] = &[
        (
            "server/setups/ApplicationAdaptersSetup.js",
            include_str!(
                "../../fixtures/behavior/authored/js/server/setups/ApplicationAdaptersSetup.js"
            ),
        ),
        (
            "server/helpers/ApplicationSettings.js",
            include_str!(
                "../../fixtures/behavior/authored/js/server/helpers/ApplicationSettings.js"
            ),
        ),
        (
            "server/helpers/ApplicationValues.js",
            include_str!("../../fixtures/behavior/authored/js/server/helpers/ApplicationValues.js"),
        ),
        (
            "server/helpers/BehaviorChecksHelper.js",
            include_str!(
                "../../fixtures/behavior/authored/js/server/helpers/BehaviorChecksHelper.js"
            ),
        ),
        (
            "server/remotes/CatalogClient.js",
            include_str!("../../fixtures/behavior/authored/js/server/remotes/CatalogClient.js"),
        ),
        (
            "server/setups/CatalogSetup.js",
            include_str!("../../fixtures/behavior/authored/js/server/setups/CatalogSetup.js"),
        ),
        (
            "server/helpers/CatalogHelper.js",
            include_str!("../../fixtures/behavior/authored/js/server/helpers/CatalogHelper.js"),
        ),
        (
            "server/helpers/InputService.js",
            include_str!("../../fixtures/behavior/authored/js/server/helpers/InputService.js"),
        ),
        (
            "server/helpers/ProductInput.js",
            include_str!("../../fixtures/behavior/authored/js/server/helpers/ProductInput.js"),
        ),
        (
            "server/entities/ProductEntity.js",
            include_str!("../../fixtures/behavior/authored/js/server/entities/ProductEntity.js"),
        ),
        (
            "server/stores/ProductQueriesStore.js",
            include_str!(
                "../../fixtures/behavior/authored/js/server/stores/ProductQueriesStore.js"
            ),
        ),
    ];
    const PY: &[(&str, &str)] = &[
        (
            "server/setups/AdaptersSetup.py",
            include_str!("../../fixtures/behavior/authored/py/server/setups/AdaptersSetup.py"),
        ),
        (
            "server/helpers/ApplicationValues.py",
            include_str!("../../fixtures/behavior/authored/py/server/helpers/ApplicationValues.py"),
        ),
        (
            "server/helpers/Backend.py",
            include_str!("../../fixtures/behavior/authored/py/server/helpers/Backend.py"),
        ),
        (
            "server/remotes/CatalogClient.py",
            include_str!("../../fixtures/behavior/authored/py/server/remotes/CatalogClient.py"),
        ),
        (
            "server/setups/CatalogSetup.py",
            include_str!("../../fixtures/behavior/authored/py/server/setups/CatalogSetup.py"),
        ),
        (
            "server/services/CatalogService.py",
            include_str!("../../fixtures/behavior/authored/py/server/services/CatalogService.py"),
        ),
        (
            "server/helpers/HandlerChecks.py",
            include_str!("../../fixtures/behavior/authored/py/server/helpers/HandlerChecks.py"),
        ),
        (
            "server/helpers/ProductInput.py",
            include_str!("../../fixtures/behavior/authored/py/server/helpers/ProductInput.py"),
        ),
        (
            "server/helpers/ProductSummaries.py",
            include_str!("../../fixtures/behavior/authored/py/server/helpers/ProductSummaries.py"),
        ),
        (
            "server/helpers/Resilient.py",
            include_str!("../../fixtures/behavior/authored/py/server/helpers/Resilient.py"),
        ),
        (
            "server/helpers/Settings.py",
            include_str!("../../fixtures/behavior/authored/py/server/helpers/Settings.py"),
        ),
        (
            "server/helpers/ValidationService.py",
            include_str!("../../fixtures/behavior/authored/py/server/helpers/ValidationService.py"),
        ),
        (
            "server/entities/ProductEntity.py",
            include_str!("../../fixtures/behavior/authored/py/server/entities/ProductEntity.py"),
        ),
        (
            "server/stores/ProductsStore.py",
            include_str!("../../fixtures/behavior/authored/py/server/stores/ProductsStore.py"),
        ),
        (
            "server/controllers/ProductsController.py",
            include_str!(
                "../../fixtures/behavior/authored/py/server/controllers/ProductsController.py"
            ),
        ),
    ];
    const PHP: &[(&str, &str)] = &[
        (
            "server/setups/AdaptersSetup.php",
            include_str!("../../fixtures/behavior/authored/php/server/setups/AdaptersSetup.php"),
        ),
        (
            "server/helpers/ApplicationValues.php",
            include_str!(
                "../../fixtures/behavior/authored/php/server/helpers/ApplicationValues.php"
            ),
        ),
        (
            "server/helpers/Backend.php",
            include_str!("../../fixtures/behavior/authored/php/server/helpers/Backend.php"),
        ),
        (
            "server/remotes/CatalogClient.php",
            include_str!("../../fixtures/behavior/authored/php/server/remotes/CatalogClient.php"),
        ),
        (
            "server/setups/CatalogSetup.php",
            include_str!("../../fixtures/behavior/authored/php/server/setups/CatalogSetup.php"),
        ),
        (
            "server/services/CatalogService.php",
            include_str!("../../fixtures/behavior/authored/php/server/services/CatalogService.php"),
        ),
        (
            "server/helpers/HandlerChecks.php",
            include_str!("../../fixtures/behavior/authored/php/server/helpers/HandlerChecks.php"),
        ),
        (
            "server/helpers/ProductInput.php",
            include_str!("../../fixtures/behavior/authored/php/server/helpers/ProductInput.php"),
        ),
        (
            "server/helpers/ProductSummaries.php",
            include_str!(
                "../../fixtures/behavior/authored/php/server/helpers/ProductSummaries.php"
            ),
        ),
        (
            "server/helpers/Resilient.php",
            include_str!("../../fixtures/behavior/authored/php/server/helpers/Resilient.php"),
        ),
        (
            "server/helpers/Settings.php",
            include_str!("../../fixtures/behavior/authored/php/server/helpers/Settings.php"),
        ),
        (
            "server/helpers/ValidationService.php",
            include_str!(
                "../../fixtures/behavior/authored/php/server/helpers/ValidationService.php"
            ),
        ),
        (
            "server/entities/ProductEntity.php",
            include_str!("../../fixtures/behavior/authored/php/server/entities/ProductEntity.php"),
        ),
        (
            "server/stores/ProductsStore.php",
            include_str!("../../fixtures/behavior/authored/php/server/stores/ProductsStore.php"),
        ),
        (
            "server/controllers/ProductsController.php",
            include_str!(
                "../../fixtures/behavior/authored/php/server/controllers/ProductsController.php"
            ),
        ),
    ];
    const RB: &[(&str, &str)] = &[
        (
            "server/setups/AdaptersSetup.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/setups/AdaptersSetup.rb"),
        ),
        (
            "server/helpers/ApplicationValues.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/helpers/ApplicationValues.rb"),
        ),
        (
            "server/helpers/Backend.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/helpers/Backend.rb"),
        ),
        (
            "server/helpers/HandlerChecks.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/helpers/HandlerChecks.rb"),
        ),
        (
            "server/helpers/ProductInput.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/helpers/ProductInput.rb"),
        ),
        (
            "server/helpers/ProductSummaries.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/helpers/ProductSummaries.rb"),
        ),
        (
            "server/helpers/Resilient.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/helpers/Resilient.rb"),
        ),
        (
            "server/helpers/Settings.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/helpers/Settings.rb"),
        ),
        (
            "server/helpers/ValidationService.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/helpers/ValidationService.rb"),
        ),
        (
            "server/remotes/CatalogClient.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/remotes/CatalogClient.rb"),
        ),
        (
            "server/setups/CatalogSetup.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/setups/CatalogSetup.rb"),
        ),
        (
            "server/services/CatalogService.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/services/CatalogService.rb"),
        ),
        (
            "server/entities/ProductEntity.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/entities/ProductEntity.rb"),
        ),
        (
            "server/stores/ProductsStore.rb",
            include_str!("../../fixtures/behavior/authored/rb/server/stores/ProductsStore.rb"),
        ),
        (
            "server/controllers/ProductsController.rb",
            include_str!(
                "../../fixtures/behavior/authored/rb/server/controllers/ProductsController.rb"
            ),
        ),
    ];

    const JAVA: &[(&str, &str)] = &[
        (
            "server/setups/RuntimeAdaptersSetup.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/setups/RuntimeAdaptersSetup.java"
            ),
        ),
        (
            "server/setups/SelectedAdaptersSetup.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/setups/SelectedAdaptersSetup.java"
            ),
        ),
        (
            "server/helpers/ApplicationValues.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/helpers/ApplicationValues.java"
            ),
        ),
        (
            "server/remotes/CatalogClient.java",
            include_str!("../../fixtures/behavior/authored/java/server/remotes/CatalogClient.java"),
        ),
        (
            "server/services/CatalogService.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/services/CatalogService.java"
            ),
        ),
        (
            "server/helpers/Input.java",
            include_str!("../../fixtures/behavior/authored/java/server/helpers/Input.java"),
        ),
        (
            "server/helpers/ProductSummaries.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/helpers/ProductSummaries.java"
            ),
        ),
        (
            "server/helpers/Resilient.java",
            include_str!("../../fixtures/behavior/authored/java/server/helpers/Resilient.java"),
        ),
        (
            "server/helpers/Settings.java",
            include_str!("../../fixtures/behavior/authored/java/server/helpers/Settings.java"),
        ),
        (
            "server/helpers/Validator.java",
            include_str!("../../fixtures/behavior/authored/java/server/helpers/Validator.java"),
        ),
        (
            "server/helpers/ProductChecksHelper.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/helpers/ProductChecksHelper.java"
            ),
        ),
        (
            "server/entities/ProductEntity.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/entities/ProductEntity.java"
            ),
        ),
        (
            "server/stores/ProductQueriesStore.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/stores/ProductQueriesStore.java"
            ),
        ),
        (
            "server/controllers/ProductsController.java",
            include_str!(
                "../../fixtures/behavior/authored/java/server/controllers/ProductsController.java"
            ),
        ),
    ];
    const GROOVY: &[(&str, &str)] = &[
        (
            "server/setups/RuntimeAdaptersSetup.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/setups/RuntimeAdaptersSetup.groovy"
            ),
        ),
        (
            "server/setups/SelectedAdaptersSetup.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/setups/SelectedAdaptersSetup.groovy"
            ),
        ),
        (
            "server/helpers/ApplicationValues.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/helpers/ApplicationValues.groovy"
            ),
        ),
        (
            "server/remotes/CatalogClient.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/remotes/CatalogClient.groovy"
            ),
        ),
        (
            "server/services/CatalogService.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/services/CatalogService.groovy"
            ),
        ),
        (
            "server/helpers/Input.groovy",
            include_str!("../../fixtures/behavior/authored/groovy/server/helpers/Input.groovy"),
        ),
        (
            "server/helpers/ProductSummaries.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/helpers/ProductSummaries.groovy"
            ),
        ),
        (
            "server/helpers/Resilient.groovy",
            include_str!("../../fixtures/behavior/authored/groovy/server/helpers/Resilient.groovy"),
        ),
        (
            "server/helpers/Settings.groovy",
            include_str!("../../fixtures/behavior/authored/groovy/server/helpers/Settings.groovy"),
        ),
        (
            "server/helpers/Validator.groovy",
            include_str!("../../fixtures/behavior/authored/groovy/server/helpers/Validator.groovy"),
        ),
        (
            "server/helpers/ProductChecksHelper.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/helpers/ProductChecksHelper.groovy"
            ),
        ),
        (
            "server/entities/ProductEntity.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/entities/ProductEntity.groovy"
            ),
        ),
        (
            "server/stores/ProductQueriesStore.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/stores/ProductQueriesStore.groovy"
            ),
        ),
        (
            "server/controllers/ProductsController.groovy",
            include_str!(
                "../../fixtures/behavior/authored/groovy/server/controllers/ProductsController.groovy"
            ),
        ),
    ];
    const KT: &[(&str, &str)] = &[
        (
            "server/setups/RuntimeAdaptersSetup.kt",
            include_str!(
                "../../fixtures/behavior/authored/kt/server/setups/RuntimeAdaptersSetup.kt"
            ),
        ),
        (
            "server/setups/SelectedAdaptersSetup.kt",
            include_str!(
                "../../fixtures/behavior/authored/kt/server/setups/SelectedAdaptersSetup.kt"
            ),
        ),
        (
            "server/helpers/ApplicationValues.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/helpers/ApplicationValues.kt"),
        ),
        (
            "server/remotes/CatalogClient.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/remotes/CatalogClient.kt"),
        ),
        (
            "server/services/CatalogService.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/services/CatalogService.kt"),
        ),
        (
            "server/helpers/Input.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/helpers/Input.kt"),
        ),
        (
            "server/helpers/ProductSummaries.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/helpers/ProductSummaries.kt"),
        ),
        (
            "server/helpers/Resilient.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/helpers/Resilient.kt"),
        ),
        (
            "server/helpers/Settings.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/helpers/Settings.kt"),
        ),
        (
            "server/helpers/Validator.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/helpers/Validator.kt"),
        ),
        (
            "server/helpers/ProductChecksHelper.kt",
            include_str!(
                "../../fixtures/behavior/authored/kt/server/helpers/ProductChecksHelper.kt"
            ),
        ),
        (
            "server/entities/ProductEntity.kt",
            include_str!("../../fixtures/behavior/authored/kt/server/entities/ProductEntity.kt"),
        ),
        (
            "server/stores/ProductQueriesStore.kt",
            include_str!(
                "../../fixtures/behavior/authored/kt/server/stores/ProductQueriesStore.kt"
            ),
        ),
        (
            "server/controllers/ProductsController.kt",
            include_str!(
                "../../fixtures/behavior/authored/kt/server/controllers/ProductsController.kt"
            ),
        ),
    ];
    const RS: &[(&str, &str)] = &[
        (
            "server/setups/ConfigSetup.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/setups/ConfigSetup.rs"),
        ),
        (
            "server/setups/SelectedConfigSetup.rs",
            include_str!(
                "../../fixtures/behavior/authored/rs/server/setups/SelectedConfigSetup.rs"
            ),
        ),
        (
            "server/helpers/ApplicationValues.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/helpers/ApplicationValues.rs"),
        ),
        (
            "server/helpers/CatalogChecks.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/helpers/CatalogChecks.rs"),
        ),
        (
            "server/remotes/CatalogClient.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/remotes/CatalogClient.rs"),
        ),
        (
            "server/services/CatalogService.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/services/CatalogService.rs"),
        ),
        (
            "server/helpers/Ledger.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/helpers/Ledger.rs"),
        ),
        (
            "server/helpers/Memory.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/helpers/Memory.rs"),
        ),
        (
            "server/helpers/ProductInput.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/helpers/ProductInput.rs"),
        ),
        (
            "server/helpers/PureValidator.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/helpers/PureValidator.rs"),
        ),
        (
            "server/helpers/Resilient.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/helpers/Resilient.rs"),
        ),
        (
            "server/helpers/ProductSummaryMapper.rs",
            include_str!(
                "../../fixtures/behavior/authored/rs/server/helpers/ProductSummaryMapper.rs"
            ),
        ),
        (
            "server/helpers/Settings.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/helpers/Settings.rs"),
        ),
        (
            "server/entities/ProductEntity.rs",
            include_str!("../../fixtures/behavior/authored/rs/server/entities/ProductEntity.rs"),
        ),
        (
            "server/stores/ProductQueriesStore.rs",
            include_str!(
                "../../fixtures/behavior/authored/rs/server/stores/ProductQueriesStore.rs"
            ),
        ),
        (
            "server/controllers/ProductsController.rs",
            include_str!(
                "../../fixtures/behavior/authored/rs/server/controllers/ProductsController.rs"
            ),
        ),
    ];
    const CS: &[(&str, &str)] = &[
        (
            "server/setups/AnnotatedConditionalConfigSetup.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/setups/AnnotatedConditionalConfigSetup.cs"
            ),
        ),
        (
            "server/setups/AnnotatedConfigSetup.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/setups/AnnotatedConfigSetup.cs"
            ),
        ),
        (
            "server/remotes/AnnotatedCatalogClient.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/remotes/AnnotatedCatalogClient.cs"
            ),
        ),
        (
            "server/services/AnnotatedCatalogService.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/services/AnnotatedCatalogService.cs"
            ),
        ),
        (
            "server/helpers/AnnotatedInput.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/AnnotatedInput.cs"),
        ),
        (
            "server/helpers/AnnotatedPageInput.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/helpers/AnnotatedPageInput.cs"
            ),
        ),
        (
            "server/helpers/AnnotatedResilient.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/helpers/AnnotatedResilient.cs"
            ),
        ),
        (
            "server/helpers/AnnotatedSettings.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/AnnotatedSettings.cs"),
        ),
        (
            "server/helpers/AnnotatedSummaries.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/helpers/AnnotatedSummaries.cs"
            ),
        ),
        (
            "server/remotes/AnnotatedValidatedCacheClient.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/remotes/AnnotatedValidatedCacheClient.cs"
            ),
        ),
        (
            "server/services/AnnotatedValidatedCacheService.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/services/AnnotatedValidatedCacheService.cs"
            ),
        ),
        (
            "server/helpers/AnnotatedValidationService.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/helpers/AnnotatedValidationService.cs"
            ),
        ),
        (
            "server/helpers/AnnotatedValues.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/AnnotatedValues.cs"),
        ),
        (
            "server/helpers/ApplicationProperties.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/helpers/ApplicationProperties.cs"
            ),
        ),
        (
            "server/helpers/CatalogExercises.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/CatalogExercises.cs"),
        ),
        (
            "server/helpers/Config.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/Config.cs"),
        ),
        (
            "server/helpers/ConfigurationAnnotationTests.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/helpers/ConfigurationAnnotationTests.cs"
            ),
        ),
        (
            "server/helpers/GuardedConfig.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/GuardedConfig.cs"),
        ),
        (
            "server/helpers/InvalidProperties.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/InvalidProperties.cs"),
        ),
        (
            "server/helpers/MissingConfig.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/MissingConfig.cs"),
        ),
        (
            "server/helpers/Product.cs",
            include_str!("../../fixtures/behavior/authored/cs/server/helpers/Product.cs"),
        ),
        (
            "server/helpers/ValidationAnnotationTests.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/helpers/ValidationAnnotationTests.cs"
            ),
        ),
        (
            "server/entities/AnnotatedProductEntity.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/entities/AnnotatedProductEntity.cs"
            ),
        ),
        (
            "server/stores/AnnotatedQueriesStore.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/stores/AnnotatedQueriesStore.cs"
            ),
        ),
        (
            "server/controllers/ProductsController.cs",
            include_str!(
                "../../fixtures/behavior/authored/cs/server/controllers/ProductsController.cs"
            ),
        ),
    ];
    const GO: &[(&str, &str)] = &[
        (
            "server/setups/ConfigSetup.go",
            include_str!("../../fixtures/behavior/authored/go/server/setups/ConfigSetup.go"),
        ),
        (
            "server/setups/SelectedConfigSetup.go",
            include_str!(
                "../../fixtures/behavior/authored/go/server/setups/SelectedConfigSetup.go"
            ),
        ),
        (
            "server/helpers/ApplicationValues.go",
            include_str!("../../fixtures/behavior/authored/go/server/helpers/ApplicationValues.go"),
        ),
        (
            "server/helpers/CatalogChecks.go",
            include_str!("../../fixtures/behavior/authored/go/server/helpers/CatalogChecks.go"),
        ),
        (
            "server/remotes/CatalogClient.go",
            include_str!("../../fixtures/behavior/authored/go/server/remotes/CatalogClient.go"),
        ),
        (
            "server/services/CatalogService.go",
            include_str!("../../fixtures/behavior/authored/go/server/services/CatalogService.go"),
        ),
        (
            "server/helpers/Ledger.go",
            include_str!("../../fixtures/behavior/authored/go/server/helpers/Ledger.go"),
        ),
        (
            "server/helpers/Memory.go",
            include_str!("../../fixtures/behavior/authored/go/server/helpers/Memory.go"),
        ),
        (
            "server/helpers/ProductInput.go",
            include_str!("../../fixtures/behavior/authored/go/server/helpers/ProductInput.go"),
        ),
        (
            "server/helpers/PureValidator.go",
            include_str!("../../fixtures/behavior/authored/go/server/helpers/PureValidator.go"),
        ),
        (
            "server/helpers/Resilient.go",
            include_str!("../../fixtures/behavior/authored/go/server/helpers/Resilient.go"),
        ),
        (
            "server/helpers/ProductSummaryMapper.go",
            include_str!(
                "../../fixtures/behavior/authored/go/server/helpers/ProductSummaryMapper.go"
            ),
        ),
        (
            "server/helpers/Settings.go",
            include_str!("../../fixtures/behavior/authored/go/server/helpers/Settings.go"),
        ),
        (
            "server/entities/ProductEntity.go",
            include_str!("../../fixtures/behavior/authored/go/server/entities/ProductEntity.go"),
        ),
        (
            "server/stores/ProductQueriesStore.go",
            include_str!(
                "../../fixtures/behavior/authored/go/server/stores/ProductQueriesStore.go"
            ),
        ),
        (
            "server/controllers/ProductsController.go",
            include_str!(
                "../../fixtures/behavior/authored/go/server/controllers/ProductsController.go"
            ),
        ),
    ];
}
