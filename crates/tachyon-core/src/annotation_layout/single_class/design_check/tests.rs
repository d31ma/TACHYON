#![allow(clippy::expect_used)]

use super::DesignCheck;
use super::DesignSwitches;
use super::project_words::ProjectWords;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

fn found(code: u16, files: &[(&str, &str)]) -> Vec<Diagnostic> {
    found_with(code, files, ProjectWords::default())
}

fn found_with(code: u16, files: &[(&str, &str)], words: ProjectWords) -> Vec<Diagnostic> {
    let pairs = files
        .iter()
        .map(|(path, text)| (Path::new(*path), text.as_bytes()));
    DesignCheck::diagnostics(pairs, &DesignSwitches::every_realm(words))
        .into_iter()
        .filter(|problem| problem.code.number() == code)
        .collect()
}

fn messages(problems: &[Diagnostic]) -> Vec<String> {
    problems
        .iter()
        .map(|problem| problem.message.clone())
        .collect()
}

#[test]
fn implicit_members_used_by_a_view_stay_public() {
    for (extension, source) in [
        (
            "java",
            "@Service\nclass CounterService {\n public int count = 0;\n public void increment() { count++; }\n}\n",
        ),
        (
            "cs",
            "[Service]\nclass CounterService {\n public int count = 0;\n public void increment() { count++; }\n}\n",
        ),
        (
            "dart",
            "@Service\nclass CounterService {\n int count = 0;\n void increment() { count++; }\n}\n",
        ),
        (
            "kt",
            "@Service\nclass CounterService {\n var count: Int = 0\n fun increment() { count++ }\n}\n",
        ),
        (
            "swift",
            "@Service\nclass CounterService {\n var count: Int = 0\n func increment() { count += 1 }\n}\n",
        ),
    ] {
        let relative = format!("client/services/CounterService.{extension}");
        let problems = found(
            1333,
            &[
                (&relative, source),
                (
                    "client/pages/tac.html",
                    "<button onclick=\"counterService.increment()\">{{counterService.count}}</button>",
                ),
            ],
        );
        assert!(problems.is_empty(), "{extension}: {problems:?}");
    }
}

#[test]
fn a_javascript_cycle_names_every_class_of_the_loop() {
    let order = "@Service\n@NeedsAll\nexport default class OrderService {\n  paymentService;\n}\n";
    let payment =
        "@Service\n@NeedsAll\nexport default class PaymentService {\n  refundService;\n}\n";
    let refund = "@Service\n@NeedsAll\nexport default class RefundService {\n  orderService;\n}\n";
    let problems = found(
        1327,
        &[
            ("server/services/OrderService.js", order),
            ("server/services/PaymentService.js", payment),
            ("server/services/RefundService.js", refund),
        ],
    );
    assert_eq!(
        messages(&problems),
        [
            "Circular dependency: OrderService -> PaymentService -> RefundService -> OrderService. Each class in this loop needs the next one, so none of them can be built first."
        ]
    );
    assert_eq!(problems[0].spans[0].file, "server/services/OrderService.js");
    let help = problems[0].help.as_deref().unwrap_or("");
    assert!(help.contains("publish an event"), "{help}");
    assert!(
        help.contains("`dependencies.check` in server/application.json"),
        "{help}"
    );
    let fixed = found(
        1327,
        &[
            ("server/services/OrderService.js", order),
            ("server/services/PaymentService.js", payment),
            (
                "server/services/RefundService.js",
                "@Service\nexport default class RefundService {\n  refund(order) { return order; }\n}\n",
            ),
        ],
    );
    assert!(fixed.is_empty(), "{fixed:?}");
}

#[test]
fn typed_ruby_and_python_cycles_are_found_through_types_and_snake_case_fields() {
    let java_order = "@Service\n@NeedsAll\nclass OrderService {\n  final StockService stock;\n}\n";
    let java_stock = "@Service\n@NeedsAll\nclass StockService {\n  final OrderService orders;\n}\n";
    let java = found(
        1327,
        &[
            ("server/services/OrderService.java", java_order),
            ("server/services/StockService.java", java_stock),
        ],
    );
    assert_eq!(java.len(), 1, "{java:?}");
    assert!(
        java[0]
            .message
            .contains("OrderService -> StockService -> OrderService")
    );
    let ruby_order =
        "#[Service]\n#[NeedsAll]\nclass OrderService\n  attr_reader :stock_service\nend\n";
    let ruby_stock =
        "#[Service]\n#[NeedsAll]\nclass StockService\n  attr_reader :order_service\nend\n";
    let ruby = found(
        1327,
        &[
            ("server/services/OrderService.rb", ruby_order),
            ("server/services/StockService.rb", ruby_stock),
        ],
    );
    assert_eq!(ruby.len(), 1, "{ruby:?}");
    let python_order = "@service()\n@needs_all()\nclass OrderService:\n    stock: StockService\n";
    let python_stock = "@service()\nclass StockService:\n    def count(self):\n        return 0\n";
    let python = found(
        1327,
        &[
            ("server/services/OrderService.py", python_order),
            ("server/services/StockService.py", python_stock),
        ],
    );
    assert!(python.is_empty(), "{python:?}");
}

#[test]
fn go_and_rust_layers_point_down() {
    let store = "// @Store\ntype OrderStore struct {\n\tprices *PriceService\n}\n";
    let service = "// @Service\ntype PriceService struct{}\n";
    let problems = found(
        1327,
        &[
            ("server/stores/OrderStore.go", store),
            ("server/services/PriceService.go", service),
        ],
    );
    assert_eq!(
        messages(&problems),
        [
            "OrderStore is a store, and it needs PriceService, which is a service. Dependencies point down: controllers, then services, then delegates, then stores, repositories and clients."
        ]
    );
    let rust_service =
        "#[Service]\n#[NeedsAll]\nstruct PriceService {\n    orders: Arc<OrdersController>,\n}\n";
    let rust_controller = "#[RequestRoute(\"/orders\")]\n#[Public]\nstruct OrdersController {}\n";
    let rust = found(
        1327,
        &[
            ("server/services/PriceService.rs", rust_service),
            ("server/controllers/OrdersController.rs", rust_controller),
        ],
    );
    assert_eq!(
        messages(&rust),
        [
            "PriceService needs OrdersController, but OrdersController is a controller, and nothing may need a controller."
        ]
    );
    let allowed = found(
        1327,
        &[
            (
                "server/controllers/OrdersController.go",
                "// @RequestRoute(\"/orders\")\n// @Public\ntype OrdersController struct {\n\tprices *PriceService\n\tstore *OrderStore\n}\n",
            ),
            ("server/services/PriceService.go", service),
            (
                "server/stores/OrderStore.go",
                "// @Store\ntype OrderStore struct{}\n",
            ),
        ],
    );
    assert!(allowed.is_empty(), "{allowed:?}");
}

#[test]
fn a_delegate_sits_between_services_and_data_access() {
    let helper = "@Delegate\n@NeedsAll\nexport default class PriceDelegate {\n  orderStore;\n  taxDelegate;\n}\n";
    let tax = "@Delegate\nexport default class TaxDelegate {}\n";
    let store = "@Store\nexport default class OrderStore {}\n";
    let service = "@Service\n@NeedsAll\nexport default class CartService {\n  priceDelegate;\n}\n";
    let allowed = found(
        1327,
        &[
            ("server/delegates/PriceDelegate.js", helper),
            ("server/delegates/TaxDelegate.js", tax),
            ("server/stores/OrderStore.js", store),
            ("server/services/CartService.js", service),
        ],
    );
    assert!(allowed.is_empty(), "{allowed:?}");
    let upward = "@Delegate\n@NeedsAll\nexport default class TaxDelegate {\n  cartService;\n}\n";
    let store_up = "@Store\n@NeedsAll\nexport default class OrderStore {\n  taxDelegate;\n}\n";
    let refused = found(
        1327,
        &[
            ("server/delegates/TaxDelegate.js", upward),
            ("server/stores/OrderStore.js", store_up),
            (
                "server/services/CartService.js",
                "@Service\nexport default class CartService {}\n",
            ),
        ],
    );
    assert_eq!(
        messages(&refused),
        [
            "OrderStore is a store, and it needs TaxDelegate, which is a delegate. Dependencies point down: controllers, then services, then delegates, then stores, repositories and clients.",
        ]
    );
}

#[test]
fn a_delegate_may_need_the_service_it_takes_on() {
    let delegate = "@Delegate\nexport default class TaxDelegate {\n  @Needs\n  taxService;\n}\n";
    let service =
        "@Service\nexport default class TaxService {\n  rate() {\n    return 2;\n  }\n}\n";
    let allowed = found(
        1327,
        &[
            ("server/delegates/TaxDelegate.js", delegate),
            ("server/services/TaxService.js", service),
        ],
    );
    assert!(allowed.is_empty(), "{allowed:?}");
}

#[test]
fn tac_services_have_their_own_realm_and_the_client_switch() {
    let cart = "@Service\n@NeedsAll\nexport default class CartService {\n  priceService;\n}\n";
    let price = "@Service\n@NeedsAll\nexport default class PriceService {\n  cartService;\n}\n";
    let problems = found(
        1327,
        &[
            ("client/services/CartService.js", cart),
            ("client/services/PriceService.js", price),
            (
                "server/services/PriceService.js",
                "@Service\nexport default class PriceService {}\n",
            ),
        ],
    );
    assert_eq!(problems.len(), 1, "{problems:?}");
    let help = problems[0].help.as_deref().unwrap_or("");
    assert!(
        help.contains("`tac.dependencies.check` in client/manifest.json"),
        "{help}"
    );
}

#[test]
fn clipped_words_are_refused_with_the_whole_word_in_every_language_family() {
    let cases = [
        (
            "server/services/CartService.js",
            "@Service\nexport default class CartService {\n  #usrCount = 0;\n  total(items) { return items; }\n}\n",
            "The field name `usrCount` in CartService is not plain English: `usr` is short for `user`.",
            "Rename it to `userCount`",
        ),
        (
            "server/services/CartService.java",
            "@Service\nclass CartService {\n  int total(int itemIdx) { return itemIdx; }\n}\n",
            "The parameter name `itemIdx` in CartService is not plain English: `idx` is short for `index`.",
            "Rename it to `itemIndex`",
        ),
        (
            "server/services/CartService.py",
            "@service()\nclass CartService:\n    def calc_total(self, items):\n        return items\n",
            "The method name `calc_total` in CartService is not plain English: `calc` is short for `calculate`.",
            "Rename it to `calculate_total`",
        ),
        (
            "server/services/CartService.rb",
            "#[Service]\nclass CartService\n  attr_reader :cfg\n  def total(items)\n    items\n  end\nend\n",
            "The field name `cfg` in CartService is not plain English: `cfg` is short for `config`.",
            "Rename it to `config`",
        ),
        (
            "server/services/CartService.go",
            "// @Service\ntype CartService struct {\n\tmsgCount int\n}\n\nfunc (service *CartService) Total(items []int) int {\n\treturn len(items)\n}\n",
            "The field name `msgCount` in CartService is not plain English: `msg` is short for `message`.",
            "Rename it to `messageCount`",
        ),
        (
            "server/services/CartService.rs",
            "#[Service]\nstruct CartService {}\n\nimpl CartService {\n    pub fn total(&self, tmp_items: Vec<u32>) -> usize {\n        tmp_items.len()\n    }\n}\n",
            "The parameter name `tmp_items` in CartService is not plain English: `tmp` is short for `temporary`.",
            "Rename it to `temporary_items`",
        ),
    ];
    for (path, source, message, rename) in cases {
        let problems = found(1328, &[(path, source)]);
        assert_eq!(messages(&problems), [message], "{path}");
        let help = problems[0].help.as_deref().unwrap_or("");
        assert!(help.contains(rename), "{path}: {help}");
    }
}

#[test]
fn plain_names_compound_words_and_project_words_pass() {
    let source = "@Service\nexport default class CheckoutService {\n  #pageLinks = [];\n  webhookUrl = '';\n  totalFor(username, zorblax) { return username + zorblax; }\n}\n";
    let unknown = found(1328, &[("server/services/CheckoutService.js", source)]);
    assert_eq!(
        messages(&unknown),
        [
            "The parameter name `zorblax` in CheckoutService is not plain English: `zorblax` is not an English word."
        ]
    );
    let help = unknown[0].help.as_deref().unwrap_or("");
    assert!(
        help.contains("`names.words` in server/application.json"),
        "{help}"
    );
    let listed = found_with(
        1328,
        &[("server/services/CheckoutService.js", source)],
        ProjectWords::of(&[], &["zorblax"]),
    );
    assert!(listed.is_empty(), "{listed:?}");
    let british = "export default class Catalogue {\n  colour = '';\n  behaviour = '';\n  licence = '';\n  organise(centre, labelled) { return centre; }\n}\n";
    let spelled = found(1328, &[("server/models/Catalogue.js", british)]);
    assert!(spelled.is_empty(), "{spelled:?}");
    let overridden = "@Service\nclass Money {\n  @Override\n  public String toStr(int idx) { return \"\"; }\n}\n";
    let skipped = found(1328, &[("server/models/Money.java", overridden)]);
    assert!(skipped.is_empty(), "{skipped:?}");
}

#[test]
fn tac_utilities_in_shared_utils_and_nested_folders_are_checked() {
    let source =
        "export default class PriceUtils {\n  static calcTotal(items) { return items; }\n}\n";
    for path in [
        "client/shared/utils/PriceUtils.js",
        "client/shared/utils/money/PriceUtils.js",
    ] {
        let problems = found(1328, &[(path, source)]);
        assert_eq!(
            messages(&problems),
            [
                "The method name `calcTotal` in PriceUtils is not plain English: `calc` is short for `calculate`."
            ],
            "{path}"
        );
    }
    let outside = found(1328, &[("client/shared/data/PriceUtils.js", source)]);
    assert!(outside.is_empty(), "{outside:?}");
}

#[test]
fn stock_keeping_units_have_a_supported_plural_parameter_name() {
    let source = "[RequestRoute(\"/ticks\")]\n[Public]\nclass TicksController {\n  [GetRoute]\n  public string[] GetTicks(string[] skus) { return skus; }\n}\n";
    let problems = found(1328, &[("server/controllers/TicksController.cs", source)]);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_method_takes_at_most_four_parameters_and_request_values_do_not_count() {
    let cases = [
        (
            "server/services/OrderService.ts",
            "@Service\nexport default class OrderService {\n  place(user: User, cart: Cart, address: Address, coupon: Coupon, note: string) { return user; }\n}\n",
        ),
        (
            "server/services/OrderService.py",
            "@service()\nclass OrderService:\n    def place(self, user, cart, address, coupon, note):\n        return user\n",
        ),
        (
            "server/services/OrderService.rb",
            "#[Service]\nclass OrderService\n  def place(user, cart, address, coupon, note)\n    user\n  end\nend\n",
        ),
        (
            "server/services/OrderService.go",
            "// @Service\ntype OrderService struct{}\n\nfunc (service *OrderService) Place(user, cart, address, coupon string, note string) string {\n\treturn user\n}\n",
        ),
        (
            "server/services/OrderService.cs",
            "[Service]\nclass OrderService {\n  public string Place(User user, Cart cart, Address address, Coupon coupon, string note) { return note; }\n}\n",
        ),
    ];
    for (path, source) in cases {
        let problems = found(1329, &[(path, source)]);
        assert_eq!(problems.len(), 1, "{path}: {problems:?}");
        assert!(
            problems[0]
                .message
                .contains("takes 5 parameters; the limit is 4."),
            "{path}"
        );
        let help = problems[0].help.as_deref().unwrap_or("");
        assert!(help.contains("a class in server/models"), "{help}");
    }
    let route = "@RequestRoute('/orders')\n@Public\nexport default class OrdersController {\n  @GetRoute('/{id}')\n  getOrder(@FromPath id, @FromQuery page, @FromQuery size, @FromHeader language, @FromCookie theme) { return id; }\n}\n";
    let allowed = found(1329, &[("server/controllers/OrdersController.js", route)]);
    assert!(allowed.is_empty(), "{allowed:?}");
    let receiver = "#[Service]\nstruct PriceService {}\n\nimpl PriceService {\n    pub fn total(&self, first: u32, second: u32, third: u32, fourth: u32) -> u32 {\n        first + second + third + fourth\n    }\n}\n";
    let counted = found(1329, &[("server/services/PriceService.rs", receiver)]);
    assert!(counted.is_empty(), "{counted:?}");
}

#[test]
fn a_class_needs_at_most_five_other_classes() {
    let names = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta"];
    let fields: String = names
        .iter()
        .map(|name| format!("  final {name}Store {}Store;\n", name.to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .concat();
    let service = format!("@Service\n@NeedsAll\nclass ReportService {{\n{fields}}}\n");
    let stores: Vec<(String, String)> = names
        .iter()
        .map(|name| {
            (
                format!("server/stores/{name}Store.java"),
                format!("@Store\nclass {name}Store {{}}\n"),
            )
        })
        .collect();
    let mut files: Vec<(&str, &str)> = stores
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    files.push(("server/services/ReportService.java", &service));
    let problems = found(1329, &files);
    assert_eq!(
        messages(&problems),
        [
            "ReportService needs 6 classes (AlphaStore, BetaStore, GammaStore, DeltaStore, EpsilonStore, ZetaStore); the limit is 5."
        ]
    );
    files.pop();
    let smaller = service.replace("  final ZetaStore zetaStore;\n", "");
    files.push(("server/services/ReportService.java", &smaller));
    assert!(found(1329, &files).is_empty());
}

#[test]
fn repeated_values_and_numbers_in_conditions_need_a_name_in_every_language_family() {
    let cases = [
        (
            "client/services/CartService.js",
            "@Service\nexport default class CartService {\n  add(bus) { bus.publish('cart:changed'); }\n  remove(bus) { bus.publish('cart:changed'); }\n}\n",
            "The text 'cart:changed' appears 2 times in CartService (lines 3, 4).",
            "static CART_CHANGED = 'cart:changed';",
        ),
        (
            "server/services/CartService.py",
            "@service()\nclass CartService:\n    def full(self, items):\n        return len(items) > 20\n",
            "The number 20 in a condition in CartService (line 4) has no name.",
            "LIMIT = 20",
        ),
        (
            "server/services/CartService.rb",
            "#[Service]\nclass CartService\n  def label(count)\n    count > 9 ? 'many' : 'few'\n  end\nend\n",
            "The number 9 in a condition in CartService (line 4) has no name.",
            "LIMIT = 9",
        ),
        (
            "server/services/CartService.go",
            "// @Service\ntype CartService struct{}\n\nfunc (service *CartService) Wait() int {\n\treturn 30 * 60\n}\n\nfunc (service *CartService) Later(extra int) int {\n\treturn 60 + extra\n}\n",
            "The number 60 appears 2 times in CartService (lines 5, 9).",
            "func (CartService) limit() int { return 60 }",
        ),
        (
            "server/services/CartService.java",
            "@Service\nclass CartService {\n  boolean full(int count) { return count >= 12; }\n}\n",
            "The number 12 in a condition in CartService (line 3) has no name.",
            "private static final int LIMIT = 12;",
        ),
        (
            "client/services/NoticeService.rs",
            "#[Service]\nstruct NoticeService {}\n\nimpl NoticeService {\n    fn first(&self) -> String {\n        \"received\".to_owned()\n    }\n    fn second(&self) -> String {\n        \"received\".to_owned()\n    }\n}\n",
            "The text \"received\" appears 2 times in NoticeService (lines 6, 9).",
            "const RECEIVED: &str = \"received\";",
        ),
    ];
    for (path, source, message, example) in cases {
        let problems = found(1330, &[(path, source)]);
        assert_eq!(messages(&problems), [message], "{path}");
        let help = problems[0].help.as_deref().unwrap_or("");
        assert!(help.contains(example), "{path}: {help}");
    }
}

#[test]
fn fsharp_help_shows_an_fsharp_constant_that_the_check_accepts() {
    let path = "client/services/CartService.fs";
    let repeated = "[<Service>]\ntype CartService() =\n    member this.Add(bus: Bus) : unit = bus.Publish(\"cart:changed\")\n    member this.Remove(bus: Bus) : unit = bus.Publish(\"cart:changed\")\n";
    let problems = found(1330, &[(path, repeated)]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    let help = problems[0].help.as_deref().unwrap_or("");
    assert!(
        help.contains("`static let cartChanged = \"cart:changed\"`"),
        "{help}"
    );
    let named = "[<Service>]\ntype CartService() =\n    static let cartChanged = \"cart:changed\"\n\n    member this.Add(bus: Bus) : unit = bus.Publish(cartChanged)\n    member this.Remove(bus: Bus) : unit = bus.Publish(cartChanged)\n";
    let problems = found(1330, &[(path, named)]);
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn named_values_plain_numbers_annotations_and_models_are_not_magic() {
    let named = "@Service\nexport default class CartService {\n  static CHANGED = 'cart:changed';\n  static LIMITS = [20, 40];\n  @OnEvent('cart:changed')\n  add(bus, items) { bus.publish(CartService.CHANGED); return items.length > 0 ? items[1] : -1; }\n  remove(bus, items) { bus.publish(CartService.CHANGED); return items.length - 1; }\n}\n";
    let problems = found(1330, &[("client/services/CartService.js", named)]);
    assert!(problems.is_empty(), "{problems:?}");
    let go = "// @Service\ntype CartService struct{}\n\nfunc (CartService) limit() int { return 20 }\n\nfunc (service *CartService) Full(count int) bool {\n\treturn count > service.limit()\n}\n";
    assert!(found(1330, &[("server/services/CartService.go", go)]).is_empty());
    let model = "export default class Order {\n  status = 'open';\n  label = 'open';\n}\n";
    assert!(found(1330, &[("server/models/Order.js", model)]).is_empty());
}

#[test]
fn migrate_check_lists_findings_even_when_a_switch_is_off() {
    let root = tempfile::tempdir().expect("temporary project");
    let services = root.path().join("server/services");
    std::fs::create_dir_all(&services).expect("services folder");
    std::fs::write(
        services.join("CartService.js"),
        "@Service\nexport default class CartService {\n  total(usrCount) { return usrCount; }\n}\n",
    )
    .expect("service");
    std::fs::write(
        root.path().join("server/application.json"),
        "{\"names\": {\"check\": false}}\n",
    )
    .expect("settings");
    let findings: Vec<_> = DesignCheck::migration_findings(root.path())
        .into_iter()
        .filter(|finding| finding.feature != "visibility.private_when_unused")
        .filter(|finding| finding.feature != "types.every_member")
        .collect();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].feature, "names.plain_english");
    assert!(
        findings[0]
            .detail
            .starts_with("Line 3: The parameter name `usrCount`"),
        "{}",
        findings[0].detail
    );
}

#[test]
fn the_build_refuses_a_break_when_the_realm_switch_is_on() {
    let root = tempfile::tempdir().expect("temporary project");
    let services = root.path().join("server/services");
    std::fs::create_dir_all(&services).expect("services folder");
    std::fs::write(
        services.join("CartService.js"),
        "@Service\nexport default class CartService {\n  total(usrCount) { return usrCount; }\n}\n",
    )
    .expect("service");
    std::fs::write(
        root.path().join("server/application.json"),
        "{\"names\": {\"check\": true}}\n",
    )
    .expect("settings");
    let failure = crate::ProjectDiscovery::discover(root.path())
        .expect_err("the clipped name stops the build");
    let codes: Vec<u16> = failure
        .diagnostics()
        .iter()
        .map(|problem| problem.code.number())
        .collect();
    assert!(codes.contains(&1328), "{codes:?}");
}

struct DefaultRulesProject;

impl DefaultRulesProject {
    const ORDER: &'static str = "@Service\n@NeedsAll\nclass OrderService {\n  paymentService;\n  total(usrCount, count, price, tax, items) {\n    this.send('sent');\n    this.send('sent');\n    return count;\n  }\n}\n";
    const PAYMENT: &'static str =
        "@Service\n@NeedsAll\nclass PaymentService {\n  orderService;\n}\n";
    const OFF: &'static str = r#"{"dependencies":{"check":false},"names":{"check":false},"signatures":{"check":false},"constants":{"check":false},"visibility":{"check":false}}"#;

    fn codes(client: bool, server: bool) -> Vec<(String, u16)> {
        let root = tempfile::tempdir().expect("project");
        Self::settings(root.path(), client, server);
        let directory =
            cap_std::fs::Dir::open_ambient_dir(root.path(), cap_std::ambient_authority())
                .expect("project capability");
        let switches = DesignSwitches::read(&directory);
        let sources = [
            ("client/services/OrderService.js", Self::ORDER),
            ("client/services/PaymentService.js", Self::PAYMENT),
            ("server/services/OrderService.js", Self::ORDER),
            ("server/services/PaymentService.js", Self::PAYMENT),
        ];
        let mut codes: Vec<(String, u16)> = DesignCheck::diagnostics(
            sources
                .iter()
                .map(|(path, text)| (Path::new(path), text.as_bytes())),
            &switches,
        )
        .iter()
        .map(|problem| (problem.spans[0].file.clone(), problem.code.number()))
        .collect();
        codes.sort();
        codes
    }

    fn settings(root: &Path, client: bool, server: bool) {
        std::fs::create_dir(root.join("client")).expect("client");
        std::fs::create_dir(root.join("server")).expect("server");
        if !client {
            std::fs::write(
                root.join("client/manifest.json"),
                format!(r#"{{"tac":{}}}"#, Self::OFF),
            )
            .expect("client settings");
        }
        if !server {
            std::fs::write(root.join("server/application.json"), Self::OFF)
                .expect("server settings");
        }
    }

    fn expected(realms: &[&str]) -> Vec<(String, u16)> {
        realms
            .iter()
            .flat_map(|realm| {
                [1327, 1328, 1329, 1330, 1333]
                    .map(|code| (format!("{realm}/services/OrderService.js"), code))
            })
            .collect()
    }
}

#[test]
fn all_five_design_rules_are_on_by_default_in_both_realms() {
    assert_eq!(
        DefaultRulesProject::codes(true, true),
        DefaultRulesProject::expected(&["client", "server"])
    );
}

#[test]
fn the_client_opt_out_leaves_all_server_design_rules_enabled() {
    assert_eq!(
        DefaultRulesProject::codes(false, true),
        DefaultRulesProject::expected(&["server"])
    );
}

#[test]
fn the_server_opt_out_leaves_all_client_design_rules_enabled() {
    assert_eq!(
        DefaultRulesProject::codes(true, false),
        DefaultRulesProject::expected(&["client"])
    );
}

#[test]
fn a_huge_single_token_file_is_checked_in_linear_time() {
    let huge = "x".repeat(1_048_577);
    let started = std::time::Instant::now();
    let files = [("client/services/LargeService.js", huge.as_str())];
    let problems = found(1328, &files);
    assert!(problems.is_empty(), "{problems:?}");
    let long_name = format!(
        "@Service\nexport default class LongService {{\n  total({}) {{ return 0; }}\n}}\n",
        "y".repeat(200_000)
    );
    let refused = found(
        1328,
        &[("server/services/LongService.js", long_name.as_str())],
    );
    assert_eq!(refused.len(), 1, "{refused:?}");
    // The sanitizer CI job runs this suite several times slower, so it raises the budget scale.
    let budget_scale = std::env::var("TAC_TEST_TIME_SCALE")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .max(1);
    let budget = std::time::Duration::from_secs(10) * budget_scale;
    assert!(
        started.elapsed() < budget,
        "{:?} is over the {budget:?} budget",
        started.elapsed()
    );
}

#[test]
#[ignore = "reports every design finding in the project folders that DESIGN_ROOTS lists, one per line"]
fn report_listed_projects() {
    let list = std::env::var("DESIGN_ROOTS").expect("DESIGN_ROOTS names a file list");
    let roots = std::fs::read_to_string(list).expect("root list");
    for root in roots.lines().filter(|line| !line.trim().is_empty()) {
        for finding in DesignCheck::migration_findings(Path::new(root)) {
            println!(
                "{root}/{} [{}] {}",
                finding.source, finding.feature, finding.detail
            );
        }
    }
}

#[test]
#[ignore = "lists the names that the design check reads in the project folders that DESIGN_ROOTS lists"]
fn list_declared_names() {
    let list = std::env::var("DESIGN_ROOTS").expect("DESIGN_ROOTS names a file list");
    let roots = std::fs::read_to_string(list).expect("root list");
    for root in roots.lines().filter(|line| !line.trim().is_empty()) {
        let sources = super::ProjectFiles::read(Path::new(root));
        for source in &sources {
            let Some(class) = super::ClassFile::read(source) else {
                continue;
            };
            let names: Vec<String> = super::declared_names::DeclaredNames::read(&class)
                .iter()
                .map(|name| format!("{}:{}", name.kind, class.words.text(name.at)))
                .collect();
            println!("{root}/{} {}", source.path, names.join(" "));
        }
    }
}
