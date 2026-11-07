#![allow(clippy::expect_used)]

use std::path::Path;

struct GoMarkerProject;

impl GoMarkerProject {
    const UNIT: &'static str = "client/tests/MarkerTest.go";
    const FILES: [(&'static str, &'static str); 14] = [
        (
            "client/entities/ProductEntity.go",
            r#"// @Entity({"adapter":"publicCatalog"})
// @Table("products")
type ProductEntity struct {
	// @Id
	id int64
	status string
}
"#,
        ),
        (
            "client/models/OrderInput.go",
            r#"// @Checked
type OrderInput struct {
	// @NotEmpty
	// @Length({"min":1,"max":80})
	name string
	// @Required
	// @Min(1)
	// @Max(9)
	level float64
}
"#,
        ),
        (
            "client/models/PublicValues.go",
            r#"type PublicValues struct {
	// @Setting("${tac.catalog.title:Catalog}")
	title string
}
"#,
        ),
        (
            "client/remotes/ProductsClient.go",
            r#"// @HTTPClient("/api")
type ProductsClient interface {
	// @PostRequest("/products")
	Save(
		// @RequestBody
		product BehaviorValue,
	) (BehaviorValue, error)
}
"#,
        ),
        (
            "client/services/CatalogService.go",
            r#"// @Service
// @CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
type CatalogService struct {
	behaviors *BehaviorContext
	Reads     int
}

// @Cached
func (catalog *CatalogService) Find(sku BehaviorValue) (BehaviorValue, error) {
	catalog.Reads++
	return map[string]any{"sku": sku, "reads": catalog.Reads}, nil
}

// @UpdateCache({"method":"Find"})
func (catalog *CatalogService) Write(sku BehaviorValue, reads BehaviorValue) (BehaviorValue, error) {
	return map[string]any{"sku": sku, "reads": reads}, nil
}

func (catalog *CatalogService) Page(
	// @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})
	input BehaviorValue,
) (BehaviorValue, error) {
	return input, nil
}

// @MaxAtOnce({"limit":1,"policy":"reject"})
func (catalog *CatalogService) Once(again BehaviorValue) (BehaviorValue, error) {
	if again != true {
		return "inside", nil
	}
	_, err := catalog.Once(false)
	return BehaviorErrorCode(err), nil
}
"#,
        ),
        (
            "client/services/OrderService.go",
            r"// @Service
type OrderService struct {
	behaviors *BehaviorContext
}

func (service *OrderService) Accept(
	// @Check(OrderInput)
	order BehaviorValue,
) (BehaviorValue, error) {
	return order, nil
}
",
        ),
        (
            "client/services/ProductService.go",
            r#"// @Service
type ProductService struct {
	// @Needs
	productStore *ProductStore
}

func (product *ProductService) Others() (BehaviorValue, error) {
	return product.productStore.Excluding("retired", map[string]any{})
}
"#,
        ),
        (
            "client/services/ShopService.go",
            r#"// @Service
type ShopService struct {
	// @Needs
	productsClient *ProductsClient
}

func (shop *ShopService) Add() (BehaviorValue, error) {
	return shop.productsClient.Save(map[string]any{"name": "anvil"})
}
"#,
        ),
        (
            "client/setups/CatalogSetup.go",
            r#"// @Setup
// @Settings({"prefix":"TAC_CATALOG_","defaults":{"pageSize":20,"cacheEnabled":false,"title":"Catalog"}})
// @WhenProfile("public")
// @WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
type CatalogSetup struct {
	// @Min(1)
	// @Max(100)
	pageSize     int64
	cacheEnabled bool
	title        string
}

func (CatalogSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
"#,
        ),
        (
            "client/setups/ProductsSetup.go",
            r"// @Setup
type ProductsSetup struct{}

func (ProductsSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	return NewBehaviorConfiguration(), nil
}
",
        ),
        (
            "client/setups/StoreSetup.go",
            r#"// @Setup
type StoreSetup struct{}

func (StoreSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	config := NewBehaviorConfiguration()
	config.Entities["publicCatalog"] = StoreSetup{}
	config.AuthorizeEntity = func(command *BehaviorEntityCommand, execution *BehaviorExecution) (bool, error) {
		return command.Entity == "ProductEntity", nil
	}
	return config, nil
}

func (StoreSetup) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	return []any{map[string]any{"id": 2, "status": "draft", "query": command.Query}}, nil
}
"#,
        ),
        (
            "client/stores/ProductStore.go",
            r#"// @Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"StoreSetup"})
type ProductStore interface {
	// @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
	Excluding(
		status BehaviorValue,
		// @PageDefaults
		page BehaviorValue,
	) (BehaviorValue, error)
}
"#,
        ),
        (
            "client/tests/CatalogTestSetup.go",
            r#"// @TestSetup
type CatalogTestSetup struct {
	entries map[string]*BehaviorCacheEntry
}

func (CatalogTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	setup := &CatalogTestSetup{entries: map[string]*BehaviorCacheEntry{}}
	config := NewBehaviorConfiguration()
	config.Caches["catalog"] = setup
	config.Http["products"] = setup
	config.Authorize = func(call *BehaviorAuthorizationCall, execution *BehaviorExecution) (bool, error) {
		return true, nil
	}
	return config, nil
}

func (setup *CatalogTestSetup) Get(key string, execution *BehaviorExecution) (*BehaviorCacheEntry, error) {
	return setup.entries[key], nil
}

func (setup *CatalogTestSetup) Set(key string, value BehaviorValue, write *BehaviorCacheWrite) error {
	setup.entries[key] = &BehaviorCacheEntry{Value: value, ExpiresAt: write.ExpiresAt}
	return nil
}

func (setup *CatalogTestSetup) Delete(key string, execution *BehaviorExecution) error {
	delete(setup.entries, key)
	return nil
}

func (setup *CatalogTestSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	return map[string]any{"method": request.Method, "path": request.Path, "body": request.Body}, nil
}
"#,
        ),
        (
            "client/tests/MarkerTest.go",
            r#"import "strings"

// @Test(CatalogTestSetup, {"partition":"go-markers"})
type MarkerTest struct {
	behaviors *BehaviorContext
}

func (test *MarkerTest) TestCheckedInputIsAcceptedAndEachBrokenRuleIsRefused() {
	orders := &OrderService{behaviors: test.behaviors}
	accepted, err := orders.Accept(map[string]any{"name": "Tea", "level": 2})
	Assert.Equals(nil, err)
	Assert.Equals(map[string]any{"name": "Tea", "level": float64(2)}, accepted)
	broken := map[string]BehaviorValue{
		"$.object":      "not an order",
		"name.notBlank": map[string]any{"name": "", "level": 2},
		"name.size":     map[string]any{"name": strings.Repeat("x", 81), "level": 2},
		"level.notNull": map[string]any{"name": "Tea"},
		"level.min":     map[string]any{"name": "Tea", "level": 0},
		"level.max":     map[string]any{"name": "Tea", "level": 10},
	}
	for path, value := range broken {
		_, err := orders.Accept(value)
		Assert.Equals("validation", BehaviorErrorCode(err), path)
		Assert.Equals(path, BehaviorErrorPath(err), path)
	}
}

func (test *MarkerTest) TestSettingsAndSettingBindPublishedValues() {
	published := map[string]any{"tac": map[string]any{"catalog": map[string]any{"title": "Test catalog", "page": map[string]any{"size": 5}, "cache": map[string]any{"enabled": true}}}}
	catalog, err := BehaviorsProperties[CatalogSetup](published)
	Assert.Equals(nil, err)
	Assert.Equals(map[string]any{"pageSize": float64(5), "cacheEnabled": true, "title": "Test catalog"}, catalog)
	values, err := BehaviorsValues[PublicValues](published)
	Assert.Equals(nil, err)
	Assert.Equals(map[string]any{"title": "Test catalog"}, values)
	defaults, err := BehaviorsValues[PublicValues](map[string]any{"tac": map[string]any{"other": "x"}})
	Assert.Equals(nil, err)
	Assert.Equals(map[string]any{"title": "Catalog"}, defaults)
}

func (test *MarkerTest) TestMinAndMaxRefuseABoundSettingOutOfRange() {
	for _, size := range []int{0, 101} {
		_, err := BehaviorsProperties[CatalogSetup](map[string]any{"tac": map[string]any{"catalog": map[string]any{"page": map[string]any{"size": size}}}})
		Assert.Equals("configuration-properties", BehaviorErrorCode(err))
	}
}

func (test *MarkerTest) TestWhenProfileAndWhenSettingGateTheSetup() {
	on := map[string]any{"tac": map[string]any{"catalog": map[string]any{"cache": map[string]any{"enabled": true}}}}
	off := map[string]any{"tac": map[string]any{"catalog": map[string]any{"cache": map[string]any{"enabled": false}}}}
	cases := []struct {
		profiles    []string
		application map[string]any
	}{
		{[]string{"public"}, on},
		{[]string{}, on},
		{[]string{"public"}, off},
		{[]string{"public"}, nil},
	}
	outcomes := []string{}
	for _, item := range cases {
		options := NewBehaviorOptions("gate", NewBehaviorSignal())
		options.Profiles = item.profiles
		if item.application != nil {
			options, _ = options.WithApplication(item.application)
		}
		context, err := BehaviorsCreate[CatalogSetup](options)
		if err != nil {
			outcomes = append(outcomes, BehaviorErrorCode(err))
			continue
		}
		context.Dispose()
		outcomes = append(outcomes, "open")
	}
	Assert.Equals([]string{"open", "inactive-configuration", "inactive-configuration", "inactive-configuration"}, outcomes)
}

func (test *MarkerTest) TestUpdateCacheWritesTheCachedRead() {
	catalog := &CatalogService{behaviors: test.behaviors}
	first, _ := catalog.Find("anvil")
	again, _ := catalog.Find("anvil")
	written, err := catalog.Write("anvil", 99)
	Assert.Equals(nil, err)
	after, _ := catalog.Find("anvil")
	Assert.Equals(map[string]any{"sku": "anvil", "reads": float64(1)}, first)
	Assert.Equals(first, again)
	Assert.Equals(map[string]any{"sku": "anvil", "reads": float64(99)}, written)
	Assert.Equals(written, after)
	Assert.Equals(1, catalog.Reads)
}

func (test *MarkerTest) TestPageDefaultsFillThePage() {
	catalog := &CatalogService{behaviors: test.behaviors}
	filled, err := catalog.Page(map[string]any{"page": 1})
	Assert.Equals(nil, err)
	Assert.Equals(map[string]any{"page": float64(1), "size": float64(20), "offset": float64(20), "sort": []any{}}, filled)
	sorted, _ := catalog.Page(map[string]any{"size": 5, "sort": []any{"id:desc"}})
	Assert.Equals(map[string]any{"page": float64(0), "size": float64(5), "offset": float64(0), "sort": []any{"id:desc"}}, sorted)
	_, err = catalog.Page(map[string]any{"size": 101})
	Assert.Equals("invalid", BehaviorErrorCode(err))
}

func (test *MarkerTest) TestMaxAtOnceRejectsASecondCallWhileOneRuns() {
	catalog := &CatalogService{behaviors: test.behaviors}
	nested, _ := catalog.Once(true)
	Assert.Equals("capacity", nested)
	alone, _ := catalog.Once(false)
	Assert.Equals("inside", alone)
}

func (test *MarkerTest) TestRequestBodyIsSentWithTheRequest() {
	shop := &ShopService{productsClient: NewProductsClientWith(test.behaviors)}
	sent, err := shop.Add()
	Assert.Equals(nil, err)
	Assert.Equals(map[string]any{"method": "POST", "path": "/api/products", "body": map[string]any{"name": "anvil"}}, sent)
}

func (test *MarkerTest) TestQueryReadsTheStoreWithItsPageDefaults() {
	store, err := NewProductStore()
	Assert.Equals(nil, err)
	rows, err := (&ProductService{productStore: store}).Others()
	Assert.Equals(nil, err)
	query := rows.([]any)[0].(map[string]any)["query"]
	Assert.Equals(map[string]any{
		"kind":    "find",
		"where":   []any{map[string]any{"field": "status", "operator": "ne", "value": "retired"}},
		"orderBy": []any{map[string]any{"field": "id", "direction": "desc"}},
		"page":    map[string]any{"page": float64(0), "size": float64(20), "offset": float64(0), "sort": []any{}},
	}, query)
}
"#,
        ),
    ];
    const CASES: [&'static str; 9] = [
        "TestCheckedInputIsAcceptedAndEachBrokenRuleIsRefused",
        "TestSettingsAndSettingBindPublishedValues",
        "TestMinAndMaxRefuseABoundSettingOutOfRange",
        "TestWhenProfileAndWhenSettingGateTheSetup",
        "TestUpdateCacheWritesTheCachedRead",
        "TestPageDefaultsFillThePage",
        "TestMaxAtOnceRejectsASecondCallWhileOneRuns",
        "TestRequestBodyIsSentWithTheRequest",
        "TestQueryReadsTheStoreWithItsPageDefaults",
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
fn go_units_check_bind_gate_cache_page_limit_query_and_send_with_their_markers() {
    if std::process::Command::new("go")
        .arg("version")
        .output()
        .is_err()
    {
        assert!(std::env::var_os("CI").is_none(), "CI must provide go");
        eprintln!("skipping: go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    GoMarkerProject::write(root.path());
    let report = super::run(root.path(), Path::new(GoMarkerProject::UNIT))
        .unwrap_or_else(|failure| panic!("{failure}"));
    let outcomes: Vec<_> = report
        .cases
        .iter()
        .map(|case| (case.name.as_str(), case.passed))
        .collect();
    assert_eq!(
        outcomes,
        GoMarkerProject::CASES.map(|name| (name, true)),
        "{report:?}"
    );
}
