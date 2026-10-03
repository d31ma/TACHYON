use super::DesignCheck;
use super::DesignSwitches;
use super::project_words::ProjectWords;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

struct VisibilityProject;

impl VisibilityProject {
    const CODE: u16 = 1333;

    fn problems(files: &[(&str, &str)]) -> Vec<Diagnostic> {
        let pairs = files
            .iter()
            .map(|(path, text)| (Path::new(*path), text.as_bytes()));
        let switches = DesignSwitches::every_realm(ProjectWords::default());
        DesignCheck::diagnostics(pairs, &switches)
            .into_iter()
            .filter(|problem| problem.code.number() == Self::CODE)
            .collect()
    }

    fn messages(files: &[(&str, &str)]) -> Vec<String> {
        Self::problems(files)
            .into_iter()
            .map(|problem| problem.message)
            .collect()
    }

    fn names(files: &[(&str, &str)]) -> Vec<String> {
        Self::names_in(files, "")
    }

    fn names_in(files: &[(&str, &str)], path: &str) -> Vec<String> {
        Self::problems(files)
            .iter()
            .filter(|problem| problem.spans[0].file.starts_with(path))
            .filter_map(|problem| problem.message.split('`').nth(1).map(str::to_owned))
            .collect()
    }

    fn built_codes(settings: Option<&str>) -> Vec<u16> {
        let root = tempfile::tempdir().expect("project");
        std::fs::create_dir_all(root.path().join("server")).expect("server");
        if let Some(settings) = settings {
            std::fs::write(root.path().join("server/application.json"), settings)
                .expect("settings");
        }
        let directory =
            cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
                .expect("project capability");
        let switches = DesignSwitches::read(&directory);
        let source =
            "@Service\nexport default class HelpService {\n  shown() {\n    return 1\n  }\n}\n";
        DesignCheck::diagnostics(
            [(
                Path::new("server/services/HelpService.js"),
                source.as_bytes(),
            )]
            .into_iter(),
            &switches,
        )
        .iter()
        .map(|problem| problem.code.number())
        .collect()
    }
}

#[test]
fn a_tac_member_stays_public_only_when_the_view_or_an_annotation_uses_it() {
    let service = "@Service\n@NeedsAll\nexport default class CartService {\n  static LIMIT = 3\n  items\n  count = 0\n  note = ''\n\n  total() {\n    return this._sum(this.items)\n  }\n\n  label() {\n    return `${this.count} of ${CartService.LIMIT}`\n  }\n\n  @OnEvent\n  changed(value) {\n    this.count = value.count\n  }\n\n  hotState() {\n    return this.count\n  }\n\n  accept(@Check order) {\n    return order\n  }\n\n  _sum(items) {\n    return items.length\n  }\n}\n";
    let view = "<p>{cart.total()} of {cart.count}</p>\n";
    let files = [
        ("client/services/CartService.js", service),
        ("client/components/cart/tac.html", view),
    ];
    assert_eq!(
        VisibilityProject::messages(&files),
        [
            "The field `LIMIT` in CartService is public, but nothing outside CartService uses it.",
            "The field `note` in CartService is public, but nothing outside CartService uses it.",
            "The method `label` in CartService is public, but nothing outside CartService uses it.",
        ]
    );
    let problems = VisibilityProject::problems(&files);
    let help = problems[0].help.as_deref().unwrap_or("");
    assert!(help.contains("`_total`"), "{help}");
    assert!(
        help.contains("`tac.visibility.check` in client/manifest.json"),
        "{help}"
    );
}

#[test]
fn another_class_keeps_a_member_public_but_a_test_does_not() {
    let price = "@Service\nexport default class PriceService {\n  total(prices: number[]): number {\n    return this.sum(prices)\n  }\n\n  round(value: number): number {\n    return Math.round(value)\n  }\n\n  private sum(prices: number[]): number {\n    return prices.length\n  }\n}\n";
    let order = "@Service\n@NeedsAll\nexport default class OrderService {\n  priceService: PriceService\n\n  checkout(prices: number[]): number {\n    return this.priceService.total(prices)\n  }\n}\n";
    let controller = "@Controller\n@NeedsAll\nexport default class OrderController {\n  orderService: OrderService\n\n  @GetRoute('/orders')\n  list(): number {\n    return this.orderService.checkout([])\n  }\n}\n";
    let test = "@Test\nexport default class PriceServiceTest {\n  priceService: PriceService\n\n  rounds(): number {\n    return this.priceService.round(1)\n  }\n}\n";
    let files = [
        ("server/services/PriceService.ts", price),
        ("server/services/OrderService.ts", order),
        ("server/controllers/OrderController.ts", controller),
        ("server/tests/PriceServiceTest.ts", test),
    ];
    assert_eq!(VisibilityProject::names(&files), ["round"]);
}

#[test]
fn every_language_writes_its_own_private_form() {
    let caller = "@Service\n@NeedsAll\nexport default class UserService {\n  helpService\n\n  help() {\n    return this.helpService.shown()\n  }\n}\n";
    let services = [
        (
            "js",
            "@Service\nexport default class HelpService {\n  extra = 1\n  #limit = 2\n\n  @OnStart\n  start() {\n    this.#limit = 3\n  }\n\n  shown() {\n    return this.#inner()\n  }\n\n  spare() {\n    return this.#limit\n  }\n\n  #inner() {\n    return this.extra\n  }\n}\n",
        ),
        (
            "ts",
            "@Service\nexport default class HelpService {\n  extra: number = 1\n  private limit: number = 2\n\n  @OnStart\n  start(): void {\n    this.limit = 3\n  }\n\n  shown(): number {\n    return this.inner()\n  }\n\n  spare(): number {\n    return this.limit\n  }\n\n  private inner(): number {\n    return this.extra\n  }\n}\n",
        ),
        (
            "py",
            "@Service\nclass HelpService:\n    extra: int = 1\n    _limit: int = 2\n\n    @OnStart\n    def start(self) -> None:\n        self._limit = 3\n\n    def shown(self) -> int:\n        return self._inner()\n\n    def spare(self) -> int:\n        return self._limit\n\n    def _inner(self) -> int:\n        return self.extra\n",
        ),
        (
            "rb",
            "#[Service]\nclass HelpService\n  attr_reader :extra\n\n  #[OnStart]\n  def start\n    @extra = 1\n  end\n\n  def shown\n    inner\n  end\n\n  def spare\n    extra\n  end\n\n  private\n\n  attr_reader :limit\n\n  def inner\n    limit\n  end\nend\n",
        ),
        (
            "go",
            "// @Service\ntype HelpService struct{}\n\n// @OnStart\nfunc (service *HelpService) Start() {\n}\n\nfunc (service *HelpService) Shown() int {\n\treturn service.inner()\n}\n\nfunc (service *HelpService) Spare() int {\n\treturn 1\n}\n\nfunc (service *HelpService) inner() int {\n\treturn 2\n}\n",
        ),
        (
            "rs",
            "#[Service]\n#[derive(Default)]\nstruct HelpService {}\n\nimpl HelpService {\n    #[OnStart]\n    pub fn start(&mut self) {}\n\n    pub fn shown(&self) -> i32 {\n        self.inner()\n    }\n\n    pub fn spare(&self) -> i32 {\n        1\n    }\n\n    fn inner(&self) -> i32 {\n        2\n    }\n}\n",
        ),
        (
            "java",
            "@Service\npublic class HelpService {\n  public int extra = 1;\n  private int limit = 2;\n\n  @OnStart\n  public void start() {\n    limit = 3;\n  }\n\n  public int shown() {\n    return inner();\n  }\n\n  public int spare() {\n    return limit;\n  }\n\n  private int inner() {\n    return extra;\n  }\n}\n",
        ),
        (
            "kt",
            "@Service\nclass HelpService {\n    var extra: Int = 1\n    private var limit: Int = 2\n\n    @OnStart\n    fun start() {\n        limit = 3\n    }\n\n    fun shown(): Int {\n        return inner()\n    }\n\n    fun spare(): Int {\n        return limit\n    }\n\n    private fun inner(): Int {\n        return extra\n    }\n}\n",
        ),
        (
            "cs",
            "[Service]\npublic class HelpService\n{\n    public int Extra = 1;\n    private int limit = 2;\n\n    [OnStart]\n    public void Start()\n    {\n        limit = 3;\n    }\n\n    public int Shown()\n    {\n        return Inner();\n    }\n\n    public int Spare()\n    {\n        return limit;\n    }\n\n    private int Inner()\n    {\n        return Extra;\n    }\n}\n",
        ),
        (
            "swift",
            "@Service\nclass HelpService {\n    var extra: Int = 1\n    private var limit: Int = 2\n\n    @OnStart\n    func start() {\n        limit = 3\n    }\n\n    func shown() -> Int {\n        return inner()\n    }\n\n    func spare() -> Int {\n        return limit\n    }\n\n    private func inner() -> Int {\n        return extra\n    }\n}\n",
        ),
        (
            "php",
            "<?php\n\n#[Service]\nclass HelpService\n{\n    public int $extra = 1;\n    private int $limit = 2;\n\n    #[OnStart]\n    public function start(): void\n    {\n        $this->limit = 3;\n    }\n\n    public function shown(): int\n    {\n        return $this->inner();\n    }\n\n    public function spare(): int\n    {\n        return $this->limit;\n    }\n\n    private function inner(): int\n    {\n        return $this->extra;\n    }\n}\n",
        ),
        (
            "dart",
            "@Service\nclass HelpService {\n  int extra = 1;\n  int _limit = 2;\n\n  @OnStart\n  void start() {\n    _limit = 3;\n  }\n\n  int shown() {\n    return _inner();\n  }\n\n  int spare() {\n    return _limit;\n  }\n\n  int _inner() {\n    return extra;\n  }\n}\n",
        ),
    ];
    for (language, source) in services {
        let path = format!("server/services/HelpService.{language}");
        let files = [
            (path.as_str(), source),
            ("server/services/UserService.js", caller),
        ];
        let names = VisibilityProject::names_in(&files, &path);
        let spare = ["spare", "Spare"][usize::from(["go", "cs"].contains(&language))];
        let expected: Vec<&str> = match language {
            "go" | "rs" => vec![spare],
            "cs" => vec!["Extra", spare],
            _ => vec!["extra", spare],
        };
        assert_eq!(names, expected, "{language}");
    }
}

#[test]
fn the_visibility_rule_is_on_until_a_realm_turns_it_off() {
    assert!(
        VisibilityProject::built_codes(None).contains(&VisibilityProject::CODE),
        "the rule starts on"
    );
    assert!(
        !VisibilityProject::built_codes(Some(r#"{"visibility":{"check":false}}"#))
            .contains(&VisibilityProject::CODE)
    );
}

#[test]
fn a_python_async_method_keeps_its_locals_out_of_the_class_fields() {
    let service = "@service\nclass HelpService:\n    async def shown(self) -> int:\n        total: int = 1\n        count = total + 1\n        return count\n";
    let controller = "@request_route(\"/help\")\n@needs_all\nclass HelpController:\n    help_service: HelpService\n\n    @get_route\n    async def get_help(self) -> int:\n        return await self.help_service.shown()\n";
    let files = [
        ("server/services/HelpService.py", service),
        ("server/controllers/HelpController.py", controller),
    ];
    assert_eq!(VisibilityProject::names(&files), Vec::<String>::new());
}

#[test]
fn an_old_go_marker_keeps_a_method_public() {
    let controller = "package main\n\n#[RequestRoute(\"/orders\")]\ntype OrdersController struct{}\n\n#[GetRoute(\"/{id}\"), ResponseStatus(201)]\nfunc (controller *OrdersController) GetOrder(id int) int {\n\treturn id\n}\n";
    let files = [("server/controllers/OrdersController.go", controller)];
    assert_eq!(VisibilityProject::names(&files), Vec::<String>::new());
}

#[test]
fn a_gateway_in_every_jvm_language_keeps_the_method_it_calls_public() {
    let service = "@service\nclass ReportService:\n    def summarize(self, period: str) -> str:\n        return period\n";
    let client = "@HTTPClient\nclass ReportClient {\n    @Gateway({\"class\":\"server/services/ReportService.py\"})\n    Object summarize(Object period) {}\n}\n";
    let controller = "@RequestRoute(\"/report\")\nclass ReportController {\n    @Needs\n    final ReportClient reportClient\n\n    @GetRoute\n    Report getReport() {\n        return new Report(reportClient.summarize(\"2026-Q3\"))\n    }\n}\n";
    for extension in ["java", "groovy"] {
        let client_path = format!("server/remotes/ReportClient.{extension}");
        let controller_path = format!("server/controllers/ReportController.{extension}");
        let files = [
            ("server/services/ReportService.py", service),
            (client_path.as_str(), client),
            (controller_path.as_str(), controller),
        ];
        assert_eq!(
            VisibilityProject::names_in(&files, "server/services"),
            Vec::<String>::new(),
            "{extension}"
        );
    }
}

#[test]
fn a_gateway_that_names_another_method_keeps_that_method_public() {
    let service = "@Service\nclass ReportService {\n    Object summarize(Object period) {\n        return period;\n    }\n}\n";
    let client = "@h_t_t_p_client\nclass ForecastClient:\n    @gateway({\"class\":\"server/services/ReportService.java\",\"method\":\"summarize\"})\n    def summary(self, period: str) -> None: ...\n";
    let controller = "@request_route(\"/forecast\")\n@needs_all()\nclass ForecastController:\n    forecast_client: ForecastClient\n\n    @get_route\n    def get_forecast(self) -> str:\n        return self.forecast_client.summary(\"2026-Q4\")\n";
    let files = [
        ("server/services/ReportService.java", service),
        ("server/remotes/ForecastClient.py", client),
        ("server/controllers/ForecastController.py", controller),
    ];
    assert_eq!(
        VisibilityProject::names_in(&files, "server/services"),
        Vec::<String>::new()
    );
}

#[test]
fn a_csharp_member_without_an_access_modifier_is_private() {
    let service = "[Service]\npublic class PriceService\n{\n    const string Currency = \"EUR\";\n    int step = 2;\n    internal int Spare = 3;\n\n    public string Shown()\n    {\n        return Label(step);\n    }\n\n    string Label(int count)\n    {\n        return Currency + count;\n    }\n}\n";
    let caller = "@Service\n@NeedsAll\nexport default class UserService {\n  priceService\n\n  help() {\n    return this.priceService.shown()\n  }\n}\n";
    let files = [
        ("server/services/PriceService.cs", service),
        ("server/services/UserService.js", caller),
    ];
    let names = VisibilityProject::names_in(&files, "server/services/PriceService.cs");
    assert_eq!(names, vec!["Spare"]);
}

#[test]
fn rust_snake_case_visibility_keeps_actual_service_and_utility_names() {
    let service = "#[Service]\n#[derive(Default)]\nstruct LabelService;\nimpl LabelService { pub fn read(&self) -> String { TitleUtils::text() } pub fn unused(&self) -> i64 { 1 } }";
    let utility = "pub struct TitleUtils;\nimpl TitleUtils { pub fn text() -> String { String::from(\"Ready\") } pub fn spare() -> i64 { 1 } }";
    let files = [
        ("client/services/nested/label_service.rs", service),
        ("client/shared/utils/nested/title_utils.rs", utility),
        ("client/components/label/tac.html", "<p>{label.read()}</p>"),
    ];
    assert_eq!(VisibilityProject::names(&files), ["unused", "spare"]);
    let messages = VisibilityProject::messages(&files);
    assert!(messages[0].contains("in LabelService"));
    assert!(messages[1].contains("in TitleUtils"));
}
