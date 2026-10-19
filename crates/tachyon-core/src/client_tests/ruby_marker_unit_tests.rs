#![allow(clippy::expect_used)]

use std::path::Path;

struct RubyMarkerProject;

impl RubyMarkerProject {
    const UNIT: &'static str = "client/tests/MarkerTest.rb";
    const FILES: [(&'static str, &'static str); 13] = [
        (
            "client/entities/ProductEntity.rb",
            r#"#[Entity({"adapter":"publicCatalog"})]
#[Table("products")]
class ProductEntity
  # @return [Integer]
  #[Id]
  attr_reader :id

  # @return [String]
  attr_reader :status
end
"#,
        ),
        (
            "client/models/OrderInput.rb",
            r#"#[Checked]
class OrderInput
  # @return [String]
  #[NotEmpty]
  #[Length({"min":1,"max":80})]
  attr_reader :name

  # @return [Integer]
  #[Required]
  #[Min(1)]
  #[Max(9)]
  attr_reader :level
end
"#,
        ),
        (
            "client/models/PublicValues.rb",
            r#"class PublicValues
  # @return [String]
  #[Setting("${tac.catalog.title:Catalog}")]
  attr_reader :title
end
"#,
        ),
        (
            "client/remotes/ProductsClient.rb",
            r#"#[HTTPClient("/api")]
class ProductsClient
  # @param product [Hash]
  # @return [Hash]
  #[PostRequest("/products")]
  def save(
    #[RequestBody]
    product
  )
  end
end
"#,
        ),
        (
            "client/services/CatalogService.rb",
            r#"#[Service]
#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
class CatalogService
  # @return [Integer]
  attr_reader :reads

  def initialize
    @reads = 0
  end

  # @param sku [String]
  # @return [Hash]
  #[Cached]
  def find(sku)
    @reads += 1
    { "sku" => sku, "reads" => @reads }
  end

  # @param sku [String]
  # @param reads [Integer]
  # @return [Hash]
  #[UpdateCache({"method":"find"})]
  def write(sku, reads)
    { "sku" => sku, "reads" => reads }
  end

  # @param input [Hash]
  # @return [Hash]
  def page(
    #[PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})]
    input
  )
    input
  end

  # @param again [Boolean]
  # @return [String]
  #[MaxAtOnce({"limit":1,"policy":"reject"})]
  def once(again)
    return "inside" unless again
    begin
      once(false)
      "entered twice"
    rescue BehaviorError => error
      error.behavior_code
    end
  end
end
"#,
        ),
        (
            "client/services/OrderService.rb",
            r"#[Service]
class OrderService
  # @param order [Hash]
  # @return [Hash]
  def accept(
    #[Check(OrderInput)]
    order
  )
    order
  end
end
",
        ),
        (
            "client/services/ProductService.rb",
            r#"#[Service]
class ProductService
  #[Needs]
  # @return [ProductStore]
  attr_reader :product_store

  # @return [Array<Hash>]
  def others
    product_store.excluding("retired", {})
  end
end
"#,
        ),
        (
            "client/services/ShopService.rb",
            r#"#[Service]
class ShopService
  #[Needs]
  # @return [ProductsClient]
  attr_reader :products_client

  # @return [Hash]
  def add
    products_client.save({ "name" => "anvil" })
  end
end
"#,
        ),
        (
            "client/setups/CatalogSetup.rb",
            r#"#[Setup]
#[Settings({"prefix":"TAC_CATALOG_"})]
#[WhenProfile("public")]
#[WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
class CatalogSetup
  # @return [Integer]
  #[Min(1)]
  #[Max(100)]
  attr_reader :page_size

  # @return [Boolean]
  attr_reader :cache_enabled

  # @return [String]
  attr_reader :title

  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    {}
  end
end
"#,
        ),
        (
            "client/setups/StoreSetup.rb",
            r#"#[Setup]
class StoreSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    { entities: { publicCatalog: { execute: method(:execute) } }, authorizeEntity: method(:product_entity?) }
  end

  # @param command [Hash]
  # @param execution [BehaviorExecution]
  # @return [Boolean]
  def self.product_entity?(command, execution)
    command["entity"] == "ProductEntity"
  end

  # @param command [Hash]
  # @param execution [BehaviorExecution]
  # @return [Array<Hash>]
  def self.execute(command, execution)
    [{ "id" => 2, "status" => "draft", "query" => command["query"] }]
  end
end
"#,
        ),
        (
            "client/stores/ProductStore.rb",
            r#"#[Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"StoreSetup"})]
class ProductStore < Repository
  # @param status [String]
  # @param page [Hash]
  # @return [Array<Hash>]
  #[Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
  def excluding(
    status,
    #[PageDefaults]
    page
  )
  end
end
"#,
        ),
        (
            "client/tests/CatalogTestSetup.rb",
            r#"#[TestSetup]
class CatalogTestSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    entries = {}
    cache = {
      get: lambda { |key, _execution| entries[key] || { "hit" => false } },
      set: lambda { |key, value, call| entries[key] = { "hit" => true, "value" => value, "expiresAt" => call.expires_at } },
      delete: lambda { |key, _execution| entries.delete(key) }
    }
    { caches: { catalog: cache }, http: { products: { exchange: method(:exchange) } }, authorize: method(:allow) }
  end

  # @param call [Hash]
  # @param execution [BehaviorExecution]
  # @return [Boolean]
  def self.allow(call, execution)
    true
  end

  # @param request [Hash]
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.exchange(request, execution)
    { "method" => request["method"], "path" => request["path"], "body" => request["body"] }
  end
end
"#,
        ),
        (
            "client/tests/MarkerTest.rb",
            r#"#[Test(CatalogTestSetup, {"partition":"ruby-markers"})]
class MarkerTest
  # @return [void]
  def test_checked_input_is_accepted_and_each_broken_rule_is_refused
    orders = construct(OrderService)
    Assert.equals({ "name" => "Tea", "level" => 2 }, orders.accept({ "name" => "Tea", "level" => 2 }))
    broken = [
      ["Checked", "not an order"],
      ["NotEmpty", { "name" => "", "level" => 2 }],
      ["Length", { "name" => "x" * 81, "level" => 2 }],
      ["Required", { "name" => "Tea" }],
      ["Min", { "name" => "Tea", "level" => 0 }],
      ["Max", { "name" => "Tea", "level" => 10 }]
    ]
    broken.each do |rule, value|
      rules = []
      begin
        orders.accept(value)
      rescue BehaviorRejection => error
        rules = error.violations.map { |violation| violation["rule"] }
      end
      Assert.equals(true, rules.include?(rule), rule)
    end
  end

  # @return [void]
  def test_settings_and_setting_bind_published_values
    published = { "tac" => { "catalog" => { "title" => "Test catalog", "page" => { "size" => 5 }, "cache" => { "enabled" => true } } } }
    catalog = Behaviors.properties(CatalogSetup, published)
    Assert.equals(5, catalog["page_size"])
    Assert.equals(true, catalog["cache_enabled"])
    Assert.equals("Test catalog", catalog["title"])
    Assert.equals({ "title" => "Test catalog" }, Behaviors.values(PublicValues, published))
    Assert.equals({ "title" => "Catalog" }, Behaviors.values(PublicValues, { "tac" => { "other" => "x" } }))
  end

  # @return [void]
  def test_min_and_max_refuse_a_bound_setting_out_of_range
    [0, 101].each do |size|
      code = ""
      begin
        Behaviors.properties(CatalogSetup, { "tac" => { "catalog" => { "title" => "T", "page" => { "size" => size }, "cache" => { "enabled" => true } } } })
      rescue BehaviorError => error
        code = error.behavior_code
      end
      Assert.equals("configuration-properties", code, size.to_s)
    end
  end

  # @return [void]
  def test_when_profile_and_when_setting_gate_the_setup
    on = { "tac" => { "catalog" => { "cache" => { "enabled" => true } } } }
    off = { "tac" => { "catalog" => { "cache" => { "enabled" => false } } } }
    outcomes = [[["public"], on], [[], on], [["public"], off], [["public"], {}]].map do |profiles, properties|
      begin
        Behaviors.create(CatalogSetup, partition: "gate", signal: BehaviorSignal.new, profiles: profiles, properties: properties).dispose
        "open"
      rescue BehaviorError => error
        error.behavior_code
      end
    end
    Assert.equals(["open", "inactive-configuration", "inactive-configuration", "inactive-configuration"], outcomes)
  end

  # @return [void]
  def test_update_cache_writes_the_cached_read
    catalog = construct(CatalogService)
    Assert.equals({ "sku" => "anvil", "reads" => 1 }, catalog.find("anvil"))
    Assert.equals({ "sku" => "anvil", "reads" => 1 }, catalog.find("anvil"))
    Assert.equals({ "sku" => "anvil", "reads" => 99 }, catalog.write("anvil", 99))
    Assert.equals({ "sku" => "anvil", "reads" => 99 }, catalog.find("anvil"))
    Assert.equals(1, catalog.reads)
  end

  # @return [void]
  def test_page_defaults_fill_the_page
    catalog = construct(CatalogService)
    Assert.equals({ "page" => 1, "size" => 20, "offset" => 20, "sort" => [] }, catalog.page({ "page" => 1 }))
    Assert.equals({ "page" => 0, "size" => 5, "offset" => 0, "sort" => ["id:desc"] }, catalog.page({ "size" => 5, "sort" => ["id:desc"] }))
    code = ""
    begin
      catalog.page({ "size" => 101 })
    rescue BehaviorError => error
      code = error.behavior_code
    end
    Assert.equals("invalid", code)
  end

  # @return [void]
  def test_max_at_once_rejects_a_second_call_while_one_runs
    catalog = construct(CatalogService)
    Assert.equals("capacity", catalog.once(true))
    Assert.equals("inside", catalog.once(false))
  end

  # @return [void]
  def test_query_reads_the_store_with_its_page_defaults
    expected = {
      "kind" => "find",
      "where" => [{ "field" => "status", "operator" => "ne", "value" => "retired" }],
      "orderBy" => [{ "field" => "id", "direction" => "desc" }],
      "page" => { "page" => 0, "size" => 20, "offset" => 0, "sort" => [] }
    }
    Assert.equals(expected, construct(ProductService).others.first["query"])
  end

  # @return [void]
  def test_request_body_is_sent_with_the_request
    Assert.equals({ "method" => "POST", "path" => "/api/products", "body" => { "name" => "anvil" } }, construct(ShopService).add)
  end
end
"#,
        ),
    ];
    const CASES: [&'static str; 9] = [
        "test_checked_input_is_accepted_and_each_broken_rule_is_refused",
        "test_settings_and_setting_bind_published_values",
        "test_min_and_max_refuse_a_bound_setting_out_of_range",
        "test_when_profile_and_when_setting_gate_the_setup",
        "test_update_cache_writes_the_cached_read",
        "test_page_defaults_fill_the_page",
        "test_max_at_once_rejects_a_second_call_while_one_runs",
        "test_query_reads_the_store_with_its_page_defaults",
        "test_request_body_is_sent_with_the_request",
    ];

    fn write(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }
}

#[test]
fn ruby_units_check_bind_gate_cache_page_limit_query_and_send_with_their_markers() {
    let ruby_installed = std::process::Command::new("ruby")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !ruby_installed {
        assert!(std::env::var_os("CI").is_none(), "CI must provide ruby");
        eprintln!("skipping: ruby is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    RubyMarkerProject::write(root.path());
    let report = super::run(root.path(), Path::new(RubyMarkerProject::UNIT))
        .unwrap_or_else(|failure| panic!("{failure}"));
    let outcomes: Vec<_> = report
        .cases
        .iter()
        .map(|case| (case.name.as_str(), case.passed))
        .collect();
    assert_eq!(
        outcomes,
        RubyMarkerProject::CASES.map(|name| (name, true)),
        "{report:?}"
    );
}
