use super::field_case::FieldCase;
use super::{AnnotationRules, Realm};
use std::path::PathBuf;

struct InjectedCases;

impl InjectedCases {
    const REFUSED: [(&'static str, Realm, &'static str, &'static str); 17] = [
        (
            "js",
            Realm::Yon,
            "@Service\nexport default class LedgerService {\n  @Needs\n  @Named(\"LedgerStore\")\n  Store;\n}\n",
            "`store`",
        ),
        (
            "ts",
            Realm::Yon,
            "@Service\nclass LedgerService {\n  @Needs\n  LedgerStore!: LedgerStore;\n}\n",
            "`ledgerStore`",
        ),
        (
            "java",
            Realm::Yon,
            "@Service\nclass LedgerService {\n  @Needs\n  LedgerStore LedgerStore;\n}\n",
            "`ledgerStore`",
        ),
        (
            "kt",
            Realm::Yon,
            "@Service\nclass LedgerService {\n  @Needs\n  lateinit var ledger_store: LedgerStore\n}\n",
            "`ledgerStore`",
        ),
        (
            "swift",
            Realm::Tac,
            "@Service\nclass BadgeService {\n    @Needs\n    var BadgeStore: BadgeStore!\n}\n",
            "`badgeStore`",
        ),
        (
            "dart",
            Realm::Yon,
            "@Service\n@NeedsAll\nclass LedgerService {\n  final LedgerStore LedgerStore;\n}\n",
            "`ledgerStore`",
        ),
        (
            "go",
            Realm::Yon,
            "// @Service\ntype LedgerService struct {\n\t// @Needs\n\tStore *LedgerStore\n}\n",
            "`store`",
        ),
        (
            "php",
            Realm::Yon,
            "<?php\n#[Service]\nclass LedgerService {\n  #[Needs]\n  private LedgerStore $LedgerStore;\n}\n",
            "`$ledgerStore`",
        ),
        (
            "py",
            Realm::Yon,
            "@service\nclass LedgerService:\n    LedgerStore: Annotated[LedgerStore, needs]\n",
            "`ledger_store`",
        ),
        (
            "py",
            Realm::Yon,
            "@service\n@needs_all\nclass LedgerService:\n    ledgerStore: LedgerStore\n",
            "`ledger_store`",
        ),
        (
            "rb",
            Realm::Yon,
            "#[Service]\nclass LedgerService\n  #[Needs]\n  attr_reader :LedgerStore\nend\n",
            "`ledger_store`",
        ),
        (
            "rb",
            Realm::Yon,
            "#[Service]\n#[NeedsAll]\nclass LedgerService\n  attr_reader :ledger_store, :auditDelegate\nend\n",
            "`audit_delegate`",
        ),
        (
            "rs",
            Realm::Yon,
            "#[Service]\npub struct LedgerService {\n    #[Needs]\n    ledgerStore: Arc<LedgerStore>,\n}\n",
            "`ledger_store`",
        ),
        (
            "cs",
            Realm::Yon,
            "[Service]\nclass LedgerService {\n  [Needs]\n  LedgerStore store;\n}\n",
            "`_store`",
        ),
        (
            "cs",
            Realm::Yon,
            "[Service]\n[NeedsAll]\nclass LedgerService {\n  private readonly LedgerStore LedgerStore;\n}\n",
            "`_ledgerStore`",
        ),
        (
            "js",
            Realm::Tac,
            "@Service\n@NeedsAll\nexport default class CartService {\n  CartStore;\n  title;\n}\n",
            "`cartStore`",
        ),
        (
            "ts",
            Realm::Tac,
            "@Service\n@NeedsAll\nexport default class CartService {\n  CartStore!: CartStore\n  title!: string\n}\n",
            "`cartStore`",
        ),
    ];
    const ACCEPTED: [(&'static str, Realm, &'static str); 13] = [
        (
            "cs",
            Realm::Tac,
            "[Store(typeof(ProductEntity))]\npublic sealed class ProductStore {\n    [Needs]\n    public BehaviorContext behaviors;\n}\n",
        ),
        (
            "js",
            Realm::Yon,
            "@Service\nclass LedgerController {\n  @Needs\n  @Named(\"LedgerService\")\n  service;\n}\n",
        ),
        (
            "ts",
            Realm::Tac,
            "@Service\n@NeedsAll\nexport default class CartService {\n  cartStore!: CartStore\n  Title!: string\n  Items!: Array<string>\n}\n",
        ),
        (
            "java",
            Realm::Yon,
            "@Service\nclass LedgerService {\n  @Needs\n  HTTPClient httpClient;\n  @Needs\n  LedgerStore ledgerStoreV2;\n}\n",
        ),
        (
            "dart",
            Realm::Tac,
            "@Service\nclass CartService {\n  @Needs\n  late final CartStore _cartStore;\n}\n",
        ),
        (
            "go",
            Realm::Yon,
            "// @Service\n// @NeedsAll\ntype LedgerService struct {\n\tstore *LedgerStore\n}\n",
        ),
        (
            "py",
            Realm::Yon,
            "@service\nclass LedgerService:\n    _ledger_store: Annotated[LedgerStore, needs]\n",
        ),
        (
            "rb",
            Realm::Yon,
            "#[Service]\n#[NeedsAll]\nclass LedgerService\n  attr_reader :ledger_store, :audit_delegate\nend\n",
        ),
        (
            "rs",
            Realm::Yon,
            "#[Service]\n#[NeedsAll]\npub struct LedgerService {\n    ledger_store: std::sync::Arc<LedgerStore>,\n}\n",
        ),
        (
            "cs",
            Realm::Yon,
            "[Service]\n[NeedsAll]\nclass LedgerService {\n  private readonly LedgerStore _store;\n  internal readonly AuditStore _audit;\n}\n",
        ),
        (
            "cs",
            Realm::Yon,
            "[Service]\nclass LedgerService {\n  [Needs]\n  public LedgerStore Store { get; set; }\n}\n",
        ),
        (
            "cs",
            Realm::Tac,
            "[Service]\n[NeedsAll]\npublic class LinksService\n{\n    public required object Documents;\n}\n",
        ),
        (
            "kt",
            Realm::Tac,
            "@Service\n@NeedsAll\nclass TallyService {\n    val Path: String\n}\n",
        ),
    ];
    const SPELLINGS: [(&'static str, bool, &'static str, &'static str); 7] = [
        ("js", false, "HTTPClient", "httpClient"),
        ("rb", false, "HTTPClient", "http_client"),
        ("cs", false, "HTTPClient", "_httpClient"),
        ("cs", true, "http_client", "HttpClient"),
        ("dart", false, "_CartStore", "_cartStore"),
        ("py", false, "_CartStore", "_cart_store"),
        ("php", false, "cart_store", "cartStore"),
    ];

    fn path(language: &str, realm: Realm) -> PathBuf {
        let folder = match realm {
            Realm::Tac => "client",
            Realm::Yon => "server",
        };
        PathBuf::from(format!("{folder}/services/SampleService.{language}"))
    }
}

#[test]
fn an_injected_field_uses_the_variable_case_of_its_language() {
    for (language, realm, source, expected) in InjectedCases::REFUSED {
        let path = InjectedCases::path(language, realm);
        let failure = AnnotationRules::check(&path, source).expect_err(&format!(
            "{language} accepted an injected field in the wrong case:\n{source}"
        ));
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(
            diagnostic.code.number(),
            1332,
            "{language}: {}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains(expected),
            "{language}: {}",
            diagnostic.message
        );
    }
}

#[test]
fn only_the_case_is_checked_and_props_keep_their_names() {
    for (language, realm, source) in InjectedCases::ACCEPTED {
        let path = InjectedCases::path(language, realm);
        let message = AnnotationRules::check(&path, source)
            .err()
            .map(|failure| failure.diagnostics()[0].message.clone());
        assert_eq!(message, None, "{language} refused:\n{source}");
    }
}

#[test]
fn the_fixed_name_keeps_the_words_and_changes_the_case() {
    for (language, public_member, written, fixed) in InjectedCases::SPELLINGS {
        let case = FieldCase::of(language, public_member).expect("case");
        assert_eq!(case.spelled(written), fixed, "{language} {written}");
        assert!(case.accepts(fixed), "{language} {fixed}");
    }
}
