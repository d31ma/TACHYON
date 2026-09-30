#[Settings({"prefix":"YON_DATABASE_","defaults":{"port":5432,"enabled":false,"ratio":0.5}})]
struct Settings {
    host: String,
    #[Min(1)]
    #[Max(65535)]
    port: i64,
    enabled: bool,
    #[Min(0)]
    #[Max(1)]
    ratio: f64,
}

#[allow(dead_code)]
struct ApplicationValues {
    #[Setting({"path":"yon.database.port","type":"integer","default":5432,"min":1,"max":65535})]
    database: u16,
    #[Setting("${yon.database.label:primary}")]
    label: String,
}

#[Setup]
#[WhenProfile("development", "test")]
#[WhenSetting({"name":"YON_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
struct SelectedConfig {}
impl SelectedConfig {
    fn configure(execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        assert_eq!(execution.partition, "configured", "Inactive configuration executed");
        Ok(BehaviorConfiguration::default())
    }
}

#[Setup]
struct Config {}
impl Config {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let memory = std::sync::Arc::new(Memory::default());
        let mut config = BehaviorConfiguration::default();
        config.caches.insert("products".into(), memory.clone());
        config.http.insert("catalog".into(), memory.clone());
        config.entities.insert("data".into(), memory);
        config.authorize = Some(std::sync::Arc::new(|call, execution| {
            execution.check()?;
            assert_eq!(call.cache, "products");
            assert_eq!(call.owner, "Catalog");
            Ok(execution.partition == "tenant-a")
        }));
        config.authorize_entity = Some(std::sync::Arc::new(|command, execution| {
            execution.check()?;
            assert_eq!(command.operation, "query");
            assert_eq!(command.table, "products");
            Ok(execution.partition == "tenant-a")
        }));
        Ok(config)
    }
}

#[Entity({"adapter":"data"})]
#[Table("products")]
struct Product {
    #[Id]
    id: i64,
}

#[Checked]
struct ProductInput {
    #[Required]
    #[NotEmpty]
    #[Length({"min":2,"max":20})]
    title: String,
    #[Min(1)]
    #[Max(100)]
    quantity: f64,
}

struct PureValidator {}
impl PureValidator {
    fn check(&self, #[Check(ProductInput)] input: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(input) }
}

#[CacheSettings({"name":"products","ttlMs":1000,"maxBytes":4096,"maxEntries":10})]
struct Catalog {
    #[Needs]
    behaviors: BehaviorContext,
    #[Needs]
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl Catalog {
    #[Cached]
    fn list(&self, #[PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] page: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let count = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Ok(BehaviorValue::object([("page", page), ("calls", BehaviorValue::Number(count as f64))]))
    }

    #[WebCall({"client":"catalog","method":"GET","path":"/products"})]
    fn remote(&self) -> BehaviorResult<BehaviorValue> {
        Ok(BehaviorValue::object([("query", BehaviorValue::object([("sort", "id".into())]))]))
    }

    #[WebCall({"client":"catalog","method":"GET","path":"/products","query":{"page":0}})]
    fn search(&self, page: BehaviorValue) -> BehaviorResult<BehaviorValue> {}

    #[UpdateCache({"method":"list"})]
    fn refresh(&self, page: BehaviorValue, value: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let _ = page;
        Ok(value)
    }

    #[ClearCache({"method":"list"})]
    fn invalidate(&self, page: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let _ = page;
        Ok(true.into())
    }

    #[UpdateCache({"method":"list"})]
    #[WebCall({"client":"catalog","method":"GET","path":"/products"})]
    fn refresh_remote(&self, page: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let _ = page;
        Ok(BehaviorValue::object([] as [(&str, BehaviorValue); 0]))
    }

    #[Cached]
    fn validated(&self, #[Check(ProductInput)] input: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(input) }
}

struct Ledger {
    #[Needs]
    behaviors: BehaviorContext,
}
impl Ledger {
    #[AllOrNothing({"adapter":"data"})]
    fn post(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        self.behaviors.repository::<Product>()?.save(BehaviorValue::object([("id", id)]))
    }

    #[AllOrNothing({"adapter":"data"})]
    fn refuse(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let _ = self.behaviors.repository::<Product>()?.save(BehaviorValue::object([("id", id)]))?;
        Err(BehaviorError { code: "validation", message: "the ledger refused the entry", path: None })
    }
}

struct Resilient {
    #[Needs]
    behaviors: BehaviorContext,
    #[Needs]
    attempts: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl Resilient {
    #[Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})]
    fn flaky(&self) -> BehaviorResult<BehaviorValue> {
        let count = self.attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        if count < 3 {
            return Err(BehaviorError { code: "adapter", message: "transient", path: None });
        }
        Ok(BehaviorValue::Number(count as f64))
    }

    #[MaxAtOnce({"limit":2,"policy":"block"})]
    fn bounded(&self, value: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(value) }
}

trait ProductSummaryMapper {
    #[Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})]
    fn to_summary(&self, product: BehaviorValue) -> BehaviorResult<BehaviorValue>;
}

#[Store(Product, {"fields":["id","status"],"defaultSize":20,"maxSize":100})]
struct ProductQueries {
    #[Needs]
    behaviors: BehaviorContext,
}
impl ProductQueries {
    fn find_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {}
    fn count_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {}
    fn exists_by_id(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {}

    #[Query({"operation":"find","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})]
    fn selected(&self, status: BehaviorValue, #[PageDefaults] page: BehaviorValue) -> BehaviorResult<BehaviorValue> {}
}

fn entity_log() -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
    static ENTITY_LOG: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<Vec<String>>>> = std::sync::OnceLock::new();
    ENTITY_LOG.get_or_init(Default::default).clone()
}

fn take_entity_log() -> Vec<String> {
    let log = entity_log();
    let mut entries = log.lock().expect("log");
    std::mem::take(&mut *entries)
}

#[derive(Default)]
struct Memory {
    entries: std::sync::Mutex<std::collections::BTreeMap<String, BehaviorCacheEntry>>,
}
impl BehaviorCacheAdapter for Memory {
    fn get(&self, key: &str, execution: &BehaviorExecution) -> BehaviorResult<Option<BehaviorCacheEntry>> {
        execution.check()?;
        let entries = self.entries.lock().map_err(|_| behavior_invalid())?;
        Ok(entries.get(key).cloned())
    }
    fn set(&self, key: &str, value: BehaviorValue, write: &BehaviorCacheWrite) -> BehaviorResult<()> {
        write.execution.check()?;
        let mut entries = self.entries.lock().map_err(|_| behavior_invalid())?;
        let entry = BehaviorCacheEntry { value, expires_at: write.expires_at };
        entries.insert(key.to_owned(), entry);
        Ok(())
    }
    fn delete(&self, key: &str, execution: &BehaviorExecution) -> BehaviorResult<()> {
        execution.check()?;
        let mut entries = self.entries.lock().map_err(|_| behavior_invalid())?;
        entries.remove(key);
        Ok(())
    }
}
impl BehaviorHttpAdapter for Memory {
    fn exchange(&self, request: &BehaviorHttpRequest, execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        assert_eq!(request.credentials, "omit");
        assert_eq!(request.redirect, "error");
        Ok(BehaviorValue::from(request.path.as_str()))
    }
}
impl BehaviorEntityAdapter for Memory {
    fn transaction(&self, command: &BehaviorTransactionCommand, execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        let entry = match &command.transaction {
            Some(id) => format!("{}:{id}", command.action),
            None => command.action.to_owned(),
        };
        entity_log().lock().expect("log").push(entry);
        Ok(BehaviorValue::from("tx-1"))
    }
    fn execute(&self, command: &BehaviorEntityCommand, execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        let transaction = command.transaction.as_deref().unwrap_or("none");
        let entry = format!("{}@{}", command.operation, transaction);
        entity_log().lock().expect("log").push(entry);
        let Some(query) = &command.query else {
            return Ok(BehaviorValue::object([("operation", command.operation.into()), ("table", command.table.into())]));
        };
        assert_eq!(command.operation, "query");
        Memory::answer_query(query)
    }
}
impl Memory {
    fn answer_query(query: &BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let Some(BehaviorValue::String(kind)) = query.get("kind") else {
            return Err(behavior_invalid());
        };
        if kind == "find" {
            let row = BehaviorValue::object([("id", "one".into()), ("status", "active".into())]);
            return Ok(BehaviorValue::Array(vec![row]));
        }
        if kind == "count" {
            return Ok(1.into());
        }
        if kind == "exists" {
            return Ok(true.into());
        }
        Err(behavior_invalid())
    }
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
    let input = product_input();
    assert_eq!(Behaviors::validate::<ProductInput>(input.clone())?, input);
    assert_eq!(PureValidator {}.check(input.clone())?, input);
    let invalid = PureValidator {}.check(blank_product_input()).expect_err("blank title rejected");
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
    let inactive = Behaviors::create::<SelectedConfig>(inactive_options).err().expect("inactive");
    assert_eq!(inactive.code, "inactive-configuration");
    let cache = BehaviorValue::object([("enabled", BehaviorValue::Bool(true))]);
    let application = BehaviorValue::object([("yon", BehaviorValue::object([("cache", cache)]))]);
    let mut selected = BehaviorOptions::new("configured", BehaviorSignal::new());
    selected.profiles.push("test".into());
    selected = selected.with_application(&application)?;
    Behaviors::create::<SelectedConfig>(selected)?.dispose();
    Ok(())
}

fn check_validation_before_authorization(catalog: &Catalog) -> BehaviorResult<()> {
    let input = product_input();
    let denied_context = Behaviors::create::<Config>(BehaviorOptions::new("denied", BehaviorSignal::new()))?;
    let denied_catalog = Catalog::new(denied_context, new_counter());
    let invalid = denied_catalog.validated(blank_product_input()).expect_err("validation precedes cache authorization");
    assert_eq!(invalid.code, "validation");
    let unauthorized = denied_catalog.validated(input.clone()).expect_err("valid input still requires authorization");
    assert_eq!(unauthorized.code, "unauthorized");
    assert_eq!(catalog.validated(input.clone())?, input);
    Ok(())
}

fn check_catalog_cache(context: &BehaviorContext, catalog: &Catalog) -> BehaviorResult<Vec<(String, BehaviorValue)>> {
    let page = BehaviorValue::object([("page", 2.into()), ("size", 5.into())]);
    let first = catalog.list(page.clone())?;
    let cached = catalog.list(page.clone())?;
    assert_eq!(first, cached);
    context.evict(catalog, "list", vec![page.clone()])?;
    let fresh = catalog.list(page.clone())?;
    let remote = catalog.remote()?;
    catalog.refresh(page.clone(), "replacement".into())?;
    assert_eq!(catalog.list(page.clone())?, BehaviorValue::from("replacement"));
    catalog.invalidate(page.clone())?;
    assert_eq!(catalog.list(page.clone())?.get("calls"), Some(&BehaviorValue::Number(3.0)));
    assert_eq!(catalog.refresh_remote(page.clone())?, BehaviorValue::from("/products"));
    assert_eq!(catalog.list(page)?, BehaviorValue::from("/products"));
    Ok(vec![("first".into(), first), ("cached".into(), cached), ("fresh".into(), fresh), ("remote".into(), remote)])
}

fn check_repository(context: &BehaviorContext) -> BehaviorResult<Vec<(String, BehaviorValue)>> {
    let repository = context.repository::<Product>()?;
    let saved = repository.save(BehaviorValue::object([("id", "one".into())]))?;
    let found = repository.find(empty_object())?;
    let deleted = repository.delete("one".into())?;
    Ok(vec![("saved".into(), saved), ("found".into(), found), ("deleted".into(), deleted)])
}

fn check_resilience(context: &BehaviorContext) -> BehaviorResult<()> {
    let attempts = new_counter();
    let resilient = Resilient::new(context.clone(), attempts.clone());
    assert_eq!(resilient.flaky()?, BehaviorValue::Number(3.0));
    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(resilient.bounded("ok".into())?, BehaviorValue::from("ok"));
    Ok(())
}

fn check_transactions(context: &BehaviorContext) -> BehaviorResult<()> {
    take_entity_log();
    let ledger = Ledger::new(context.clone());
    ledger.post("one".into())?;
    assert_eq!(take_entity_log(), ["begin", "save@tx-1", "commit:tx-1"]);
    assert!(ledger.refuse("two".into()).is_err(), "a refused entry must not commit");
    assert_eq!(take_entity_log(), ["begin", "save@tx-1", "rollback:tx-1"]);
    Ok(())
}

fn check_queries(context: &BehaviorContext) -> BehaviorResult<()> {
    let products = ProductQueries::new(context.clone());
    let query_rows = products.find_by_status("active".into())?;
    assert!(matches!(query_rows, BehaviorValue::Array(ref rows) if rows.len() == 1));
    assert_eq!(products.count_by_status("active".into())?, 1.into());
    assert_eq!(products.exists_by_id("one".into())?, true.into());
    let first_page = BehaviorValue::object([("size", 1.into())]);
    assert_eq!(products.selected("active".into(), first_page)?, query_rows);
    Ok(())
}

fn check_mapper(context: &BehaviorContext) -> BehaviorResult<()> {
    let summaries = ProductSummaryMapper::new();
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
    check_validation()?;
    check_settings()?;
    check_application_values()?;
    check_selected_config()?;
    let signal = BehaviorSignal::new();
    let context = Behaviors::create::<Config>(BehaviorOptions::new("tenant-a", signal.clone()))?;
    let catalog = Catalog::new(context.clone(), new_counter());
    check_validation_before_authorization(&catalog)?;
    let mut results = check_catalog_cache(&context, &catalog)?;
    results.extend(check_repository(&context)?);
    check_resilience(&context)?;
    check_transactions(&context)?;
    check_queries(&context)?;
    check_mapper(&context)?;
    signal.abort();
    assert_eq!(catalog.remote().expect_err("cancelled").code, "aborted");
    context.dispose();
    Ok(BehaviorValue::object(results))
}

struct Handler;
impl Handler {
    #[allow(non_snake_case)]
    fn GET() -> http::Response<Vec<u8>> {
        let application: BehaviorValue = Json::parse("{\"yon\":{\"server\":{\"host\":\"127.0.0.1\",\"port\":9000}}}").expect("valid configuration JSON").into();
        let bridged = Behaviors::values::<ApplicationValues>(&application);
        let database = bridged.as_ref().ok().and_then(|bound| bound.get("database"));
        if database != Some(&BehaviorValue::Number(5432.0)) {
            return Self::json_answer(500, "{\"error\":\"json-bridge\"}".to_owned());
        }
        match exercise().and_then(|value| value.to_json(1_048_576)) {
            Ok(value) => Self::json_answer(200, value),
            Err(error) => Self::json_answer(500, format!("{{\"error\":\"{}\"}}", error.code)),
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
