use super::super::DesignCheck;
use super::super::DesignSwitches;
use super::super::project_words::ProjectWords;
use std::path::Path;

struct TypedProject;

impl TypedProject {
    const CODE: u16 = 1356;

    fn messages(files: &[(&str, &str)]) -> Vec<String> {
        let pairs = files
            .iter()
            .map(|(path, text)| (Path::new(*path), text.as_bytes()));
        let switches = DesignSwitches::every_realm(ProjectWords::default());
        DesignCheck::diagnostics(pairs, &switches)
            .into_iter()
            .filter(|problem| problem.code.number() == Self::CODE)
            .map(|problem| problem.message)
            .collect()
    }

    fn helps(files: &[(&str, &str)]) -> Vec<String> {
        let pairs = files
            .iter()
            .map(|(path, text)| (Path::new(*path), text.as_bytes()));
        let switches = DesignSwitches::every_realm(ProjectWords::default());
        DesignCheck::diagnostics(pairs, &switches)
            .into_iter()
            .filter(|problem| problem.code.number() == Self::CODE)
            .filter_map(|problem| problem.help)
            .collect()
    }
}

#[test]
fn javascript_members_need_jsdoc_types() {
    let untyped = "@Service\nexport default class CartService {\n  @Needs\n  taxHelper;\n  count = 0;\n  items;\n\n  total(extra) {\n    return this.count + extra;\n  }\n\n  clear() {\n    this.count = 0;\n  }\n}\n";
    let helper = "@Helper\nexport default class TaxHelper {\n  /**\n   * @param {number} value\n   * @returns {number}\n   */\n  add(value) {\n    return value;\n  }\n}\n";
    assert_eq!(
        TypedProject::messages(&[
            ("client/services/CartService.js", untyped),
            ("client/helpers/TaxHelper.js", helper),
        ]),
        [
            "The parameter `extra` of `total` in CartService has no type.",
            "The return value of `total` in CartService has no type.",
            "The field `taxHelper` in CartService has no type.",
            "The field `items` in CartService has no type.",
            "The field `items` in CartService has no starting value, and Tachyon does not fill it.",
        ]
    );
    let typed = "@Service\nexport default class CartService {\n  /** @type {TaxHelper} */\n  @Needs\n  taxHelper;\n  count = 0;\n  /** @type {string[]} */\n  items = [];\n\n  /**\n   * @param {number} [extra]\n   * @returns {number}\n   */\n  total(extra) {\n    return this.count + extra;\n  }\n\n  clear() {\n    this.count = 0;\n  }\n}\n";
    assert!(
        TypedProject::messages(&[
            ("client/services/CartService.js", typed),
            ("client/helpers/TaxHelper.js", helper),
        ])
        .is_empty()
    );
}

#[test]
fn a_javascript_field_starts_with_a_value_or_is_filled_by_tachyon() {
    let service = "@Service\nexport default class ShelfService {\n  /** @type {ShelfStore} */\n  @Needs\n  shelfStore;\n  /** @type {string} */\n  @Setting(\"${tac.shelf.name:Main}\")\n  name;\n  /** @type {string[]} */\n  labels = [];\n  /** @type {number | undefined} */\n  chosen;\n}\n";
    let filled = "@Service\n@NeedsAll\nexport default class BoxService {\n  /** @type {ShelfStore} */\n  shelfStore;\n}\n";
    let entity = "@Entity\nclass BoxEntity {\n  /** @type {number} */\n  @Id\n  id;\n}\n";
    let typescript = "@Service\nexport default class TrayService {\n  chosen!: number;\n}\n";
    let files = [
        ("client/services/ShelfService.js", service),
        ("client/services/BoxService.js", filled),
        ("server/entities/BoxEntity.js", entity),
        ("client/services/TrayService.ts", typescript),
    ];
    assert_eq!(
        TypedProject::messages(&files),
        ["The field `chosen` in ShelfService has no starting value, and Tachyon does not fill it."]
    );
    let helps = TypedProject::helps(&files);
    assert!(
        helps[0].starts_with("Give it a starting value (`chosen = value`), or put `@Needs` above it when Tachyon should fill it."),
        "{helps:?}"
    );
}

#[test]
fn typescript_refuses_an_implicit_any() {
    let untyped = "@Service\nexport default class PriceService {\n  @Needs\n  taxHelper;\n  count = 0;\n\n  total(extra, rate: number, base = 1) {\n    return extra * rate * base;\n  }\n}\n";
    assert_eq!(
        TypedProject::messages(&[("server/services/PriceService.ts", untyped)]),
        [
            "The parameter `extra` of `total` in PriceService has no type.",
            "The field `taxHelper` in PriceService has no type.",
        ]
    );
    let typed = "@Service\nexport default class PriceService {\n  @Needs\n  taxHelper!: TaxHelper;\n  count = 0;\n\n  total(extra: number, rate: number, base = 1) {\n    return extra * rate * base;\n  }\n}\n";
    assert!(TypedProject::messages(&[("server/services/PriceService.ts", typed)]).is_empty());
}

#[test]
fn python_members_need_native_hints() {
    let untyped = "@Service()\nclass StockService:\n    LIMIT = 3\n    names = []\n\n    def __init__(self, store):\n        self.store = store\n\n    def count(self, shelf):\n        return shelf\n\n    def clear(self) -> None:\n        pass\n";
    assert_eq!(
        TypedProject::messages(&[("server/services/StockService.py", untyped)]),
        [
            "The parameter `store` of `__init__` in StockService has no type.",
            "The parameter `shelf` of `count` in StockService has no type.",
            "The return value of `count` in StockService has no type.",
            "The field `names` in StockService has no type.",
        ]
    );
    let typed = "@Service()\nclass StockService:\n    LIMIT = 3\n    names: list[str] = []\n\n    def __init__(self, store: Store):\n        self.store = store\n\n    def count(self, shelf: int) -> int:\n        return shelf\n\n    async def clear(self) -> None:\n        emptied = []\n        self.names = emptied\n";
    assert!(TypedProject::messages(&[("server/services/StockService.py", typed)]).is_empty());
}

#[test]
fn php_members_need_native_types() {
    let untyped = "<?php\n#[Service]\nclass FeeService\n{\n    const LIMIT = 3;\n    private $rate = 2;\n\n    public function total($amount)\n    {\n        return $amount * $this->rate;\n    }\n\n    public function clear()\n    {\n        $this->rate = 0;\n    }\n}\n";
    assert_eq!(
        TypedProject::messages(&[("server/services/FeeService.php", untyped)]),
        [
            "The parameter `amount` of `total` in FeeService has no type.",
            "The return value of `total` in FeeService has no type.",
            "The return value of `clear` in FeeService has no type.",
            "The field `$rate` in FeeService has no type.",
        ]
    );
    let typed = "<?php\n#[Service]\nclass FeeService\n{\n    const LIMIT = 3;\n    private int $rate = 2;\n\n    public function total(int $amount): int\n    {\n        return $amount * $this->rate;\n    }\n\n    public function clear(): void\n    {\n        $this->rate = 0;\n    }\n}\n";
    assert!(TypedProject::messages(&[("server/services/FeeService.php", typed)]).is_empty());
}

#[test]
fn ruby_members_need_yard_types_that_name_known_classes() {
    let untyped = "#[Service]\nclass BookService\n  #[Needs]\n  attr_reader :shelf_store\n\n  def initialize\n    @count = 0\n  end\n\n  def find(title)\n    title\n  end\nend\n";
    let store =
        "#[Store]\nclass ShelfStore\n  # @return [Integer]\n  def size\n    1\n  end\nend\n";
    assert_eq!(
        TypedProject::messages(&[
            ("server/services/BookService.rb", untyped),
            ("server/stores/ShelfStore.rb", store),
        ]),
        [
            "The parameter `title` of `find` in BookService has no type.",
            "The return value of `find` in BookService has no type.",
            "The field `shelf_store` in BookService has no type.",
        ]
    );
    let typed = "#[Service]\nclass BookService\n  #[Needs]\n  # @return [ShelfStore]\n  attr_reader :shelf_store\n\n  def initialize\n    @count = 0\n  end\n\n  # @param title [String]\n  # @return [Array<Book>, nil]\n  def find(title)\n    title\n  end\nend\n";
    assert_eq!(
        TypedProject::messages(&[
            ("server/services/BookService.rb", typed),
            ("server/stores/ShelfStore.rb", store),
        ]),
        ["The type `Book` in a comment of BookService names no known class."]
    );
    let book = "class Book\n  # @return [String]\n  attr_reader :title\nend\n";
    assert!(
        TypedProject::messages(&[
            ("server/services/BookService.rb", typed),
            ("server/stores/ShelfStore.rb", store),
            ("server/models/Book.rb", book),
        ])
        .is_empty()
    );
}

#[test]
fn a_class_file_cannot_switch_type_checking_off() {
    let script = "// @ts-nocheck\n@Service\nexport default class NoteService {\n  count = 0;\n\n  clear() {\n    // @ts-ignore\n    this.count = 0;\n  }\n}\n";
    let python = "# mypy: ignore-errors\n@Service()\nclass NoteService:\n    def clear(self) -> None:\n        self.count = 0  # type: ignore\n";
    assert_eq!(
        TypedProject::messages(&[
            ("client/services/NoteService.js", script),
            ("server/services/NoteService.py", python),
        ]),
        [
            "`@ts-nocheck` in NoteService switches type checking off.",
            "`@ts-ignore` in NoteService switches type checking off.",
            "`type: ignore` in NoteService switches type checking off.",
            "`mypy: ignore-errors` in NoteService switches type checking off.",
        ]
    );
    let words =
        "@Service\nexport default class NoteService {\n  label = '@ts-ignore is refused';\n}\n";
    assert!(TypedProject::messages(&[("client/services/NoteService.js", words)]).is_empty());
}

#[test]
fn the_help_shows_the_type_comment_to_add() {
    let service = "@Service\nexport default class CartService {\n  total(extra) {\n    return extra;\n  }\n}\n";
    let helps = TypedProject::helps(&[("client/services/CartService.js", service)]);
    assert!(
        helps[0].starts_with("Add `@param {T} extra` to the JSDoc comment (`/** ... */`) above it. Write the real type in place of `T`."),
        "{helps:?}"
    );
    assert!(helps[0].contains("`tac.types.check`"), "{helps:?}");
}

#[test]
fn the_rule_is_off_unless_a_project_switches_it_on() {
    let root = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(root.path().join("server")).expect("server");
    let directory = cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
        .expect("project capability");
    let source = "@Service\nexport default class HelpService {\n  shown(value) {\n    return value\n  }\n}\n";
    let codes = |switches: &DesignSwitches| -> Vec<u16> {
        DesignCheck::diagnostics(
            [(
                Path::new("server/services/HelpService.js"),
                source.as_bytes(),
            )]
            .into_iter(),
            switches,
        )
        .iter()
        .map(|problem| problem.code.number())
        .filter(|code| *code == TypedProject::CODE)
        .collect()
    };
    assert!(codes(&DesignSwitches::read(&directory)).is_empty());
    std::fs::write(
        root.path().join("server/application.json"),
        r#"{"types":{"check":true}}"#,
    )
    .expect("settings");
    assert_eq!(codes(&DesignSwitches::read(&directory)), [1356, 1356]);
}
