use super::MigrationFix;
use super::edits::TextEdit;
use super::type_comments_fix::TypeCommentsFix;
use super::visibility_off::VisibilityOff;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct TypeCases;

impl TypeCases {
    const CHANGED: [(&'static str, &'static str, &'static str); 5] = [
        (
            "server/services/CartService.js",
            "@Service\nexport default class CartService {\n  @Needs\n  taxHelper;\n\n  /** @returns {number} */\n  total() {\n    return this.taxHelper.rate();\n  }\n}\n",
            "@Service\nexport default class CartService {\n  /** @type {TaxHelper} */\n  @Needs\n  taxHelper;\n\n  /** @returns {number} */\n  total() {\n    return this.taxHelper.rate();\n  }\n}\n",
        ),
        (
            "server/services/PriceService.ts",
            "@Service\nexport default class PriceService {\n  @Needs\n  taxHelper;\n}\n",
            "@Service\nexport default class PriceService {\n  @Needs\n  taxHelper!: TaxHelper;\n}\n",
        ),
        (
            "server/services/StockService.py",
            "@Service()\nclass StockService:\n    def clear(self):\n        self.count = 0\n",
            "@Service()\nclass StockService:\n    def clear(self) -> None:\n        self.count = 0\n",
        ),
        (
            "server/services/FeeService.php",
            "<?php\n#[Service]\nclass FeeService\n{\n    #[Needs]\n    public $taxHelper;\n\n    public function clear()\n    {\n        echo 1;\n    }\n}\n",
            "<?php\n#[Service]\nclass FeeService\n{\n    #[Needs]\n    public TaxHelper $taxHelper;\n\n    public function clear(): void\n    {\n        echo 1;\n    }\n}\n",
        ),
        (
            "server/services/BookService.rb",
            "#[Service]\nclass BookService\n  #[Needs]\n  attr_reader :tax_helper\nend\n",
            "#[Service]\nclass BookService\n  #[Needs]\n  # @return [TaxHelper]\n  attr_reader :tax_helper\nend\n",
        ),
    ];
    const HELPER: (&'static str, &'static str) = (
        "server/helpers/TaxHelper.js",
        "@Helper\nexport default class TaxHelper {\n  /** @returns {number} */\n  rate() {\n    return 2;\n  }\n}\n",
    );
    const UNKNOWN: (&'static str, &'static str) = (
        "server/services/NoteService.py",
        "@Service()\nclass NoteService:\n    def shown(self, value):\n        return value\n",
    );
    const SERVER_ON: &'static str = r#"{"types":{"check":true},"visibility":{"check":false}}"#;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        fs::write(path, contents).expect("file");
    }

    fn read(root: &Path, relative: &str) -> String {
        fs::read_to_string(root.join(relative)).expect("file")
    }

    fn project(settings: Option<&str>) -> TempDir {
        let project = TempDir::new().expect("project");
        let root = project.path();
        VisibilityOff::write(root);
        for (relative, before, _) in Self::CHANGED {
            Self::write(root, relative, before);
        }
        Self::write(root, Self::HELPER.0, Self::HELPER.1);
        Self::write(root, Self::UNKNOWN.0, Self::UNKNOWN.1);
        if let Some(settings) = settings {
            Self::write(root, "server/application.json", settings);
        }
        project
    }
}

#[test]
fn known_types_are_added_and_the_rest_are_listed() {
    let project = TypeCases::project(Some(TypeCases::SERVER_ON));
    let root = project.path();
    let plan = MigrationFix::plan(root).expect("plan");
    let listed = plan.to_text(false);
    assert!(
        listed.contains("The parameter `value` of `shown` in NoteService has no type."),
        "{listed}"
    );
    assert!(
        listed.contains("The return value of `shown` in NoteService has no type."),
        "{listed}"
    );
    plan.write(root).expect("write");
    for (relative, _, after) in TypeCases::CHANGED {
        assert_eq!(TypeCases::read(root, relative), after, "{relative}");
    }
    assert_eq!(
        TypeCases::read(root, TypeCases::UNKNOWN.0),
        TypeCases::UNKNOWN.1
    );
}

#[test]
fn a_project_without_the_types_switch_is_left_alone() {
    let project = TypeCases::project(None);
    let root = project.path();
    MigrationFix::plan(root)
        .expect("plan")
        .write(root)
        .expect("write");
    for (relative, before, _) in TypeCases::CHANGED {
        assert_eq!(TypeCases::read(root, relative), before, "{relative}");
    }
}

#[test]
#[ignore = "adds the known types in the project folders that TYPE_ROOTS lists, switch or not"]
fn type_listed_projects() {
    let list = std::env::var("TYPE_ROOTS").expect("TYPE_ROOTS names a file list");
    let roots = fs::read_to_string(list).expect("root list");
    for root in roots.lines().filter(|line| !line.trim().is_empty()) {
        let root = Path::new(root);
        let missing = crate::annotation_layout::DesignCheck::every_missing_type(root);
        let mut paths: Vec<&str> = missing.iter().map(|found| found.path.as_str()).collect();
        paths.sort_unstable();
        paths.dedup();
        for relative in paths {
            let path = root.join(relative);
            let source = fs::read_to_string(&path).expect("source");
            let edits: Vec<TextEdit> = TypeCommentsFix::edits(relative, &missing)
                .into_iter()
                .map(|(edit, _)| edit)
                .collect();
            fs::write(&path, TextEdit::apply(&source, &edits)).expect("write");
        }
    }
}
