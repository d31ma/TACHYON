#[Description("Behavior conformance exercise helpers")]
struct CatalogChecks {}
impl CatalogChecks {
fn entity_log() -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
    static ENTITY_LOG: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<Vec<String>>>> = std::sync::OnceLock::new();
    ENTITY_LOG.get_or_init(Default::default).clone()
}

fn take_entity_log() -> Vec<String> {
    let log = CatalogChecks::entity_log();
    let mut entries = log.lock().expect("log");
    std::mem::take(&mut *entries)
}

fn product_input() -> BehaviorValue {
    BehaviorValue::object([("title", "é".into()), ("quantity", 2.into())])
}

fn blank_product_input() -> BehaviorValue {
    BehaviorValue::object([("title", "\u{00a0}".into())])
}

fn empty_object() -> BehaviorValue {
    BehaviorValue::object([] as [(&str, BehaviorValue); 0])
}

fn new_counter() -> std::sync::Arc<std::sync::atomic::AtomicUsize> {
    std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0))
}

fn check_validation() -> BehaviorResult<()> {
    let input = CatalogChecks::product_input();
    assert_eq!(Behaviors::validate::<ProductInput>(input.clone())?, input);
    assert_eq!(PureValidator {}.check(input.clone())?, input);
    let invalid = PureValidator {}.check(CatalogChecks::blank_product_input()).expect_err("blank title rejected");
    assert_eq!(invalid.code, "validation");
    assert_eq!(invalid.path.as_deref(), Some("title.notBlank"));
    Ok(())
}

fn check_settings() -> BehaviorResult<()> {
    let environment = std::collections::BTreeMap::from([
        ("YON_DATABASE_HOST".into(), "localhost".into()),
        ("YON_DATABASE_PORT".into(), "9000".into()),
    ]);
    let settings = Behaviors::properties::<Settings>(&environment)?;
    let database = BehaviorValue::object([("host", BehaviorValue::from("localhost")), ("port", BehaviorValue::Number(9000.0))]);
    let nested = BehaviorValue::object([("yon", BehaviorValue::object([("database", database)]))]);
    assert_eq!(Behaviors::properties::<Settings>(&nested)?, settings);
    assert_eq!(settings.get("port"), Some(&BehaviorValue::Number(9000.0)));
    assert_eq!(settings.get("enabled"), Some(&BehaviorValue::Bool(false)));
    Ok(())
}

fn check_application_values() -> BehaviorResult<()> {
    let server = BehaviorValue::object([("host", BehaviorValue::from("127.0.0.1")), ("port", BehaviorValue::Number(9000.0))]);
    let database = BehaviorValue::object([("port", BehaviorValue::Number(6543.0))]);
    let application = BehaviorValue::object([("yon", BehaviorValue::object([("server", server), ("database", database)]))]);
    let bound = Behaviors::values::<ApplicationValues>(&application)?;
    assert_eq!(bound.get("database"), Some(&BehaviorValue::Number(6543.0)));
    assert_eq!(bound.get("label"), Some(&BehaviorValue::String("primary".into())));
    let unknown = BehaviorValue::object([("secrets", BehaviorValue::object(Vec::<(&str, BehaviorValue)>::new()))]);
    let unknown_error = Behaviors::values::<ApplicationValues>(&unknown).err().expect("unknown section");
    assert_eq!(unknown_error.code, "configuration-properties");
    let manifest_database = BehaviorValue::object([("port", BehaviorValue::Number(7000.0))]);
    let manifest = BehaviorValue::object([
        ("name", BehaviorValue::from("Catalog")),
        ("yon", BehaviorValue::object([("database", manifest_database)])),
    ]);
    let manifest_values = Behaviors::values::<ApplicationValues>(&manifest)?;
    assert_eq!(manifest_values.get("database"), Some(&BehaviorValue::Number(7000.0)));
    Ok(())
}

fn check_selected_config() -> BehaviorResult<()> {
    let inactive_options = BehaviorOptions::new("must-not-execute", BehaviorSignal::new());
    let inactive = Behaviors::create::<SelectedConfigSetup>(inactive_options).err().expect("inactive");
    assert_eq!(inactive.code, "inactive-configuration");
    let cache = BehaviorValue::object([("enabled", BehaviorValue::Bool(true))]);
    let application = BehaviorValue::object([("yon", BehaviorValue::object([("cache", cache)]))]);
    let mut selected = BehaviorOptions::new("configured", BehaviorSignal::new());
    selected.profiles.push("test".into());
    selected = selected.with_application(&application)?;
    Behaviors::create::<SelectedConfigSetup>(selected)?.dispose();
    Ok(())
}

fn check_validation_before_authorization(catalog: &CatalogService) -> BehaviorResult<()> {
    let input = CatalogChecks::product_input();
    let denied_context = Behaviors::create::<ConfigSetup>(BehaviorOptions::new("denied", BehaviorSignal::new()))?;
    let denied_catalog = CatalogService { behaviors: denied_context, calls: CatalogChecks::new_counter() };
    let invalid = denied_catalog.validated(CatalogChecks::blank_product_input()).expect_err("validation precedes cache authorization");
    assert_eq!(invalid.code, "validation");
    let unauthorized = denied_catalog.validated(input.clone()).expect_err("valid input still requires authorization");
    assert_eq!(unauthorized.code, "unauthorized");
    assert_eq!(catalog.validated(input.clone())?, input);
    Ok(())
}

fn check_catalog_cache(context: &BehaviorContext, catalog: &CatalogService, client: &CatalogClient) -> BehaviorResult<Vec<(String, BehaviorValue)>> {
    let page = BehaviorValue::object([("page", 2.into()), ("size", 5.into())]);
    let first = catalog.list(page.clone())?;
    let cached = catalog.list(page.clone())?;
    assert_eq!(first, cached);
    context.evict(catalog, "list", vec![page.clone()])?;
    let fresh = catalog.list(page.clone())?;
    let remote = client.remote()?;
    catalog.refresh(page.clone(), "replacement".into())?;
    assert_eq!(catalog.list(page.clone())?, BehaviorValue::from("replacement"));
    catalog.invalidate(page.clone())?;
    assert_eq!(catalog.list(page.clone())?.get("calls"), Some(&BehaviorValue::Number(3.0)));
    assert_eq!(client.find(7.into())?, BehaviorValue::from("/products/7"));
    Ok(vec![("first".into(), first), ("cached".into(), cached), ("fresh".into(), fresh), ("remote".into(), remote)])
}

fn check_repository(context: &BehaviorContext) -> BehaviorResult<Vec<(String, BehaviorValue)>> {
    let repository = context.repository::<ProductEntity>()?;
    let saved = repository.save(BehaviorValue::object([("id", "one".into())]))?;
    let found = repository.find(CatalogChecks::empty_object())?;
    let deleted = repository.delete("one".into())?;
    Ok(vec![("saved".into(), saved), ("found".into(), found), ("deleted".into(), deleted)])
}

fn check_resilience(context: &BehaviorContext) -> BehaviorResult<()> {
    let attempts = CatalogChecks::new_counter();
    let resilient = Resilient::new(context.clone(), attempts.clone());
    assert_eq!(resilient.flaky()?, BehaviorValue::Number(3.0));
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(resilient.bounded("ok".into())?, BehaviorValue::from("ok"));
    Ok(())
}

fn check_transactions(context: &BehaviorContext) -> BehaviorResult<()> {
    CatalogChecks::take_entity_log();
    let ledger = Ledger::new(context.clone());
    ledger.post("one".into())?;
    assert_eq!(CatalogChecks::take_entity_log(), ["begin", "save@tx-1", "commit:tx-1"]);
    assert!(ledger.refuse("two".into()).is_err(), "a refused entry must not commit");
    assert_eq!(CatalogChecks::take_entity_log(), ["begin", "save@tx-1", "rollback:tx-1"]);
    Ok(())
}

fn check_queries(context: &BehaviorContext) -> BehaviorResult<()> {
    let products = ProductQueriesStore { behaviors: context.clone() };
    let query_rows = products.find_by_status("active".into())?;
    assert!(matches!(query_rows, BehaviorValue::Array(ref rows) if rows.len() == 1));
    assert_eq!(products.count_by_status("active".into())?, 1.into());
    assert_eq!(products.exists_by_id("one".into())?, true.into());
    let first_page = BehaviorValue::object([("size", 1.into())]);
    assert_eq!(products.selected("active".into(), first_page)?, query_rows);
    Ok(())
}

fn check_mapper(context: &BehaviorContext) -> BehaviorResult<()> {
    let summaries = ProductSummaryMapper::new(context.clone());
    let product = BehaviorValue::object([
        ("detail", BehaviorValue::object([("id", 7.into())])),
        ("name", "Anvil".into()),
        ("extra", "dropped".into()),
    ]);
    let expected = BehaviorValue::object([
        ("productId", 7.into()),
        ("title", "Anvil".into()),
        ("absent", BehaviorValue::Null),
        ("kind", "product".into()),
    ]);
    assert_eq!(summaries.to_summary(product)?, expected);
    let not_an_object = summaries.to_summary(BehaviorValue::Array(vec![])).expect_err("object only");
    assert_eq!(not_an_object.code, "invalid");
    Ok(())
}

fn exercise() -> BehaviorResult<BehaviorValue> {
    CatalogChecks::check_validation()?;
    CatalogChecks::check_settings()?;
    CatalogChecks::check_application_values()?;
    CatalogChecks::check_selected_config()?;
    let signal = BehaviorSignal::new();
    let context = Behaviors::create::<ConfigSetup>(BehaviorOptions::new("tenant-a", signal.clone()))?;
    let catalog = CatalogService { behaviors: context.clone(), calls: CatalogChecks::new_counter() };
    let client = CatalogClient { behaviors: context.clone() };
    CatalogChecks::check_validation_before_authorization(&catalog)?;
    let mut results = CatalogChecks::check_catalog_cache(&context, &catalog, &client)?;
    results.extend(CatalogChecks::check_repository(&context)?);
    CatalogChecks::check_resilience(&context)?;
    CatalogChecks::check_transactions(&context)?;
    CatalogChecks::check_queries(&context)?;
    CatalogChecks::check_mapper(&context)?;
    signal.abort();
    assert_eq!(client.remote().expect_err("cancelled").code, "aborted");
    context.dispose();
    Ok(BehaviorValue::object(results))
}

fn answer() -> http::Response<Vec<u8>> {
    let application: BehaviorValue = Json::parse("{\"yon\":{\"server\":{\"host\":\"127.0.0.1\",\"port\":9000}}}").expect("valid configuration JSON").into();
    let bridged = Behaviors::values::<ApplicationValues>(&application);
    let database = bridged.as_ref().ok().and_then(|bound| bound.get("database"));
    if database != Some(&BehaviorValue::Number(5432.0)) {
        return CatalogChecks::json_answer(500, "{\"error\":\"json-bridge\"}".to_owned());
    }
    match CatalogChecks::exercise().and_then(|value| value.to_json(1_048_576)) {
        Ok(value) => CatalogChecks::json_answer(200, value),
        Err(error) => CatalogChecks::json_answer(500, format!("{{\"error\":\"{}\"}}", error.code)),
    }
}

fn json_answer(status: u16, body: String) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body.into_bytes())
        .unwrap_or_default()
}
}
