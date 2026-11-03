include!("../../../src/behavior/polyglot/rust_runtime.rs");
include!("../../../src/behavior/polyglot/rust_storage.rs");

const __TACHYON_BEHAVIOR_MODULE_ID: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const __TACHYON_BEHAVIOR_ENVIRONMENT: &str = "yon";

#[derive(Default)]
struct Memory {
    entries: std::sync::Mutex<std::collections::BTreeMap<String, BehaviorCacheEntry>>,
    authorizations: std::sync::atomic::AtomicUsize,
    reads: std::sync::atomic::AtomicUsize,
}
impl BehaviorCacheAdapter for Memory {
    fn get(
        &self,
        key: &str,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<Option<BehaviorCacheEntry>> {
        execution.check()?;
        let read = self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        assert!(
            self.authorizations
                .load(std::sync::atomic::Ordering::SeqCst)
                >= read,
            "authorization runs before every read, including hits"
        );
        assert!(key.contains(__TACHYON_BEHAVIOR_MODULE_ID));
        Ok(self.entries.lock().unwrap().get(key).cloned())
    }
    fn set(
        &self,
        key: &str,
        value: BehaviorValue,
        write: &BehaviorCacheWrite,
    ) -> BehaviorResult<()> {
        write.execution.check()?;
        if write.execution.partition == "write-fails" {
            return Err(behavior_invalid());
        }
        self.entries.lock().unwrap().insert(
            key.to_owned(),
            BehaviorCacheEntry {
                value,
                expires_at: write.expires_at,
            },
        );
        Ok(())
    }
    fn delete(&self, key: &str, execution: &BehaviorExecution) -> BehaviorResult<()> {
        execution.check()?;
        self.entries.lock().unwrap().remove(key);
        Ok(())
    }
}
impl BehaviorHttpAdapter for Memory {
    fn exchange(
        &self,
        request: &BehaviorHttpRequest,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        assert_eq!(request.redirect, "error");
        assert_eq!(request.credentials, "omit");
        assert_eq!(request.max_response_bytes, 1_048_576);
        assert_eq!(request.path, "/products");
        Ok(BehaviorValue::from("transport"))
    }
}
impl BehaviorEntityAdapter for Memory {
    fn execute(
        &self,
        command: &BehaviorEntityCommand,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        assert_eq!(command.table, "products");
        let Some(query) = &command.query else {
            return Ok(BehaviorValue::from(command.operation));
        };
        Memory::execute_query(command, query, execution)
    }
}
impl Memory {
    fn execute_query(
        command: &BehaviorEntityCommand,
        query: &BehaviorValue,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue> {
        assert_ne!(
            execution.partition, "query-denied",
            "Denied query reached the adapter"
        );
        assert_eq!(command.operation, "query");
        assert!(command.value.is_none() && command.page.is_none() && command.id.is_none());
        if execution.partition == "query-bad-result" {
            return Ok("not a query result".into());
        }
        if execution.partition == "query-too-many" {
            return Ok(BehaviorValue::Array(vec![empty_object(); 101]));
        }
        match query.get("kind") {
            Some(BehaviorValue::String(kind)) if kind == "find" => {
                let id_ascending =
                    BehaviorValue::object([("field", "id".into()), ("direction", "asc".into())]);
                assert_eq!(
                    query.get("orderBy"),
                    Some(&BehaviorValue::Array(vec![id_ascending]))
                );
                let row = BehaviorValue::object([("id", "one".into())]);
                Ok(BehaviorValue::Array(vec![row]))
            }
            Some(BehaviorValue::String(kind)) if kind == "count" => Ok(1.into()),
            Some(BehaviorValue::String(kind)) if kind == "exists" => Ok(true.into()),
            _ => Err(behavior_invalid()),
        }
    }
}

struct Config;
impl BehaviorConfigurationSource for Config {
    fn behavior_configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let memory = std::sync::Arc::new(Memory::default());
        let mut config = BehaviorConfiguration::default();
        config.caches.insert("products".into(), memory.clone());
        config.http.insert("catalog".into(), memory.clone());
        config.entities.insert("data".into(), memory.clone());
        config.authorize = Some(std::sync::Arc::new(move |_, execution| {
            execution.check()?;
            memory
                .authorizations
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(execution.partition != "denied")
        }));
        config.authorize_entity = Some(std::sync::Arc::new(|command, execution| {
            execution.check()?;
            assert_eq!(command.operation, "query");
            assert_eq!(command.entity, "Product");
            if execution.partition == "query-timeout" {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(execution.partition != "query-denied")
        }));
        Ok(config)
    }
}
struct Product;
impl BehaviorEntity for Product {
    fn behavior_entity() -> BehaviorEntityMetadata {
        BehaviorEntityMetadata {
            name: "Product",
            adapter: "data",
            table: "products",
            id: "id",
        }
    }
}

fn empty_object() -> BehaviorValue {
    BehaviorValue::object([] as [(&str, BehaviorValue); 0])
}

fn create_context(partition: &str) -> BehaviorResult<BehaviorContext> {
    Behaviors::create::<Config>(BehaviorOptions::new(partition, BehaviorSignal::new()))
}

fn policy() -> BehaviorMethodPolicy {
    BehaviorMethodPolicy {
        owner: "Catalog",
        name: "list",
        cache: Some(BehaviorCachePolicy {
            name: "products",
            ttl_ms: 1000,
            max_bytes: 1024,
            max_entries: 10,
        }),
        key_args: Some(vec![0]),
        pageable: Some(BehaviorPagePolicy {
            argument: 0,
            default_size: 20,
            max_size: 100,
            sort: vec!["id"],
        }),
        http: None,
        mutation: None,
        retry: None,
        limit: None,
        transactional: None,
    }
}

fn plain_policy() -> BehaviorMethodPolicy {
    let mut policy = policy();
    policy.cache = None;
    policy.key_args = None;
    policy.pageable = None;
    policy
}

fn mutation_policy(operation: &'static str) -> BehaviorMethodPolicy {
    let mut result = policy();
    result.name = "update";
    result.key_args = None;
    result.pageable = None;
    result.mutation = Some(BehaviorCacheMutationPolicy {
        operation,
        method: "list",
        arguments: vec![0],
        key_args: vec![0],
        pageable: policy().pageable,
    });
    result
}

fn check_cache_put_and_evict(context: &BehaviorContext) -> BehaviorResult<()> {
    let calls = std::cell::Cell::new(0);
    for _ in 0..2 {
        context.invoke(mutation_policy("cache:put"), vec![empty_object()], |_| {
            calls.set(calls.get() + 1);
            Ok("replacement".into())
        })?;
    }
    assert_eq!(calls.get(), 2, "CachePut always executes the body");
    let read = || {
        context.invoke(policy(), vec![empty_object()], |_| {
            panic!("replacement cache missed")
        })
    };
    assert_eq!(read()?, BehaviorValue::from("replacement"));
    let oversized = context.invoke(mutation_policy("cache:put"), vec![empty_object()], |_| {
        Ok("x".repeat(1024).into())
    });
    assert!(oversized.is_err());
    assert_eq!(read()?, BehaviorValue::from("replacement"));
    let failed = context.invoke(mutation_policy("cache:evict"), vec![empty_object()], |_| {
        Err(behavior_invalid())
    });
    assert!(failed.is_err());
    assert_eq!(read()?, BehaviorValue::from("replacement"));
    let not_a_number = context.invoke(mutation_policy("cache:evict"), vec![empty_object()], |_| {
        Ok(BehaviorValue::Number(f64::NAN))
    });
    assert!(not_a_number.is_err());
    assert_eq!(read()?, BehaviorValue::from("replacement"));
    context.invoke(mutation_policy("cache:evict"), vec![empty_object()], |_| {
        Ok(true.into())
    })?;
    assert_eq!(
        context.invoke(policy(), vec![empty_object()], |_| Ok("fresh".into()))?,
        BehaviorValue::from("fresh")
    );
    Ok(())
}

fn check_denied_mutations() -> BehaviorResult<()> {
    let denied = create_context("denied")?;
    for operation in ["cache:put", "cache:evict"] {
        let error = denied
            .invoke(mutation_policy(operation), vec![empty_object()], |_| {
                panic!("unauthorized mutation executed")
            })
            .unwrap_err();
        assert_eq!(error.code, "unauthorized");
    }
    Ok(())
}

fn check_invalid_mutation_policies(context: &BehaviorContext) {
    let mut invalid = mutation_policy("cache:put");
    invalid.mutation.as_mut().unwrap().arguments = vec![2];
    let result = context.invoke(invalid, vec![empty_object()], |_| {
        panic!("invalid mapping executed")
    });
    assert!(result.is_err());
    let mut invalid = mutation_policy("cache:put");
    invalid.key_args = Some(vec![0]);
    let result = context.invoke(invalid, vec![empty_object()], |_| {
        panic!("Cacheable mutation executed")
    });
    assert!(result.is_err());
}

fn check_failed_cache_write() -> BehaviorResult<()> {
    let failing = create_context("write-fails")?;
    let side_effects = std::cell::Cell::new(0);
    let error = failing
        .invoke(mutation_policy("cache:put"), vec![empty_object()], |_| {
            side_effects.set(side_effects.get() + 1);
            Ok("value".into())
        })
        .unwrap_err();
    assert_eq!(error.code, "adapter");
    assert_eq!(side_effects.get(), 1, "No retry after write failure");
    Ok(())
}

fn check_interrupted_mutations() -> BehaviorResult<()> {
    let signal = BehaviorSignal::new();
    let aborted =
        Behaviors::create::<Config>(BehaviorOptions::new("abort-mutation", signal.clone()))?;
    let error = aborted
        .invoke(mutation_policy("cache:put"), vec![empty_object()], |_| {
            signal.abort();
            Ok("value".into())
        })
        .unwrap_err();
    assert_eq!(error.code, "aborted");
    let mut options = BehaviorOptions::new("deadline-mutation", BehaviorSignal::new());
    options.timeout_ms = 5;
    let deadline = Behaviors::create::<Config>(options)?;
    let error = deadline
        .invoke(mutation_policy("cache:evict"), vec![empty_object()], |_| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(true.into())
        })
        .unwrap_err();
    assert_eq!(error.code, "timeout");
    Ok(())
}

fn mutation_contracts() -> BehaviorResult<()> {
    let context = create_context("mutations")?;
    check_cache_put_and_evict(&context)?;
    check_denied_mutations()?;
    check_invalid_mutation_policies(&context);
    check_failed_cache_write()?;
    check_interrupted_mutations()
}

fn query_policy() -> BehaviorQueryPolicy {
    BehaviorQueryPolicy {
        entity: Product::behavior_entity(),
        fields: vec!["id", "status"],
        default_size: 20,
        max_size: 100,
        operation: "find",
        predicates: vec![BehaviorQueryPredicate {
            field: "status",
            operator: "eq",
            argument: Some(0),
        }],
        order_by: vec![BehaviorQuerySort {
            field: "id",
            direction: "asc",
        }],
        page_argument: None,
    }
}

fn active() -> Vec<BehaviorValue> {
    vec!["active".into()]
}

fn check_query_results(context: &BehaviorContext) -> BehaviorResult<()> {
    let rows = context.query(query_policy(), active())?;
    assert!(matches!(rows, BehaviorValue::Array(rows) if rows.len() == 1));
    let near_limit = vec![BehaviorValue::from("x".repeat(65_460))];
    let normalized = behavior_query(&query_policy(), &near_limit)?;
    assert!(normalized.get("where").unwrap().to_json(65_536).is_ok());
    assert!(normalized.to_json(1_048_576)?.len() > 65_536);
    assert!(
        context.query(query_policy(), near_limit).is_ok(),
        "Query metadata must not consume the separate binding budget"
    );
    Ok(())
}

fn check_count_and_exists(context: &BehaviorContext) -> BehaviorResult<()> {
    for operation in ["count", "exists"] {
        let mut policy = query_policy();
        policy.operation = operation;
        policy.order_by.clear();
        let expected = if operation == "count" {
            1.into()
        } else {
            true.into()
        };
        assert_eq!(context.query(policy.clone(), active())?, expected);
        let broken = create_context("query-bad-result")?;
        assert!(broken.query(policy, active()).is_err());
    }
    Ok(())
}

fn check_rejected_query_partitions() -> BehaviorResult<()> {
    for partition in ["query-bad-result", "query-too-many", "query-denied"] {
        let other = create_context(partition)?;
        assert!(other.query(query_policy(), active()).is_err());
    }
    let missing = Behaviors::create::<MissingAuthorization>(BehaviorOptions::new(
        "missing-query-auth",
        BehaviorSignal::new(),
    ))?;
    assert!(missing.query(query_policy(), active()).is_err());
    Ok(())
}

fn check_query_bindings(context: &BehaviorContext) {
    for value in [
        BehaviorValue::Number(f64::NAN),
        BehaviorValue::object([("sql", "untrusted".into())]),
        "x".repeat(65_536).into(),
    ] {
        assert!(context.query(query_policy(), vec![value]).is_err());
    }
    let mut in_policy = query_policy();
    in_policy.predicates[0].operator = "in";
    let hundred = BehaviorValue::Array(vec!["active".into(); 100]);
    assert!(context.query(in_policy.clone(), vec![hundred]).is_ok());
    let hundred_and_one = BehaviorValue::Array(vec!["active".into(); 101]);
    assert!(context.query(in_policy, vec![hundred_and_one]).is_err());
}

fn check_query_pages(context: &BehaviorContext) {
    let mut page_policy = query_policy();
    page_policy.page_argument = Some(1);
    let unknown_sort =
        BehaviorValue::object([("sort", BehaviorValue::Array(vec!["status".into()]))]);
    assert!(
        context
            .query(page_policy.clone(), vec!["active".into(), unknown_sort])
            .is_err()
    );
    let oversized = BehaviorValue::object([("size", 101.into())]);
    assert!(
        context
            .query(page_policy.clone(), vec!["active".into(), oversized])
            .is_err()
    );
    let single = BehaviorValue::object([("size", 1.into())]);
    assert!(
        context
            .query(page_policy, vec!["active".into(), single])
            .is_ok()
    );
}

fn check_invalid_query_calls(context: &BehaviorContext) {
    assert!(context.query(query_policy(), vec![]).is_err());
    assert!(
        context
            .query(query_policy(), vec!["active".into(), "unused".into()])
            .is_err()
    );
    let mut invalid = query_policy();
    invalid.predicates[0].field = "unknown";
    assert!(context.query(invalid, active()).is_err());
    let mut invalid = query_policy();
    invalid.operation = "update";
    assert!(context.query(invalid, active()).is_err());
}

fn check_interrupted_queries(context: &BehaviorContext) -> BehaviorResult<()> {
    let mut options = BehaviorOptions::new("query-timeout", BehaviorSignal::new());
    options.timeout_ms = 5;
    let deadline = Behaviors::create::<Config>(options)?;
    let error = deadline.query(query_policy(), active()).unwrap_err();
    assert_eq!(error.code, "timeout");
    let signal = BehaviorSignal::new();
    let cancelled =
        Behaviors::create::<Config>(BehaviorOptions::new("query-cancelled", signal.clone()))?;
    signal.abort();
    let error = cancelled.query(query_policy(), active()).unwrap_err();
    assert_eq!(error.code, "aborted");
    context.dispose();
    let error = context.query(query_policy(), active()).unwrap_err();
    assert_eq!(error.code, "disposed");
    Ok(())
}

fn query_contracts() -> BehaviorResult<()> {
    let context = create_context("queries")?;
    check_query_results(&context)?;
    check_count_and_exists(&context)?;
    check_rejected_query_partitions()?;
    check_query_bindings(&context);
    check_query_pages(&context);
    check_invalid_query_calls(&context);
    check_interrupted_queries(&context)
}

struct Settings;
impl BehaviorPropertiesSource for Settings {
    fn behavior_properties() -> BehaviorPropertiesPolicy {
        BehaviorPropertiesPolicy {
            prefix: "YON_APP_",
            fields: vec![
                BehaviorPropertyField {
                    field: "title",
                    name: "TITLE",
                    kind: "string",
                    default: Some("catalog".into()),
                    min: None,
                    max: None,
                },
                BehaviorPropertyField {
                    field: "enabled",
                    name: "ENABLED",
                    kind: "boolean",
                    default: Some(false.into()),
                    min: None,
                    max: None,
                },
                BehaviorPropertyField {
                    field: "limit",
                    name: "LIMIT",
                    kind: "integer",
                    default: Some(20.into()),
                    min: Some(1.0),
                    max: Some(100.0),
                },
                BehaviorPropertyField {
                    field: "ratio",
                    name: "RATIO",
                    kind: "number",
                    default: Some(BehaviorValue::Number(0.5)),
                    min: Some(0.0),
                    max: Some(1.0),
                },
            ],
        }
    }
}

struct ConditionalConfig;
impl ConditionalConfig {
    fn configure_calls() -> &'static std::sync::atomic::AtomicUsize {
        static CONFIGURE_CALLS: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        &CONFIGURE_CALLS
    }

    fn configure_call_count() -> usize {
        ConditionalConfig::configure_calls().load(std::sync::atomic::Ordering::SeqCst)
    }
}
impl BehaviorConfigurationSource for ConditionalConfig {
    fn behavior_conditions() -> BehaviorConfigurationConditions {
        BehaviorConfigurationConditions {
            profiles: vec!["development", "test"],
            property: Some(BehaviorPropertyCondition {
                name: "YON_FEATURE_ENABLED",
                having_value: "true",
                match_if_missing: false,
            }),
        }
    }
    fn behavior_configure(_: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        ConditionalConfig::configure_calls().fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(BehaviorConfiguration::default())
    }
}

fn property_values() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::from([
        (String::from("YON_APP_TITLE"), String::from("カタログ")),
        (String::from("YON_APP_ENABLED"), String::from("true")),
        (String::from("YON_APP_LIMIT"), String::from("42")),
        (String::from("YON_APP_RATIO"), String::from("1e-1")),
    ])
}

fn check_property_binding(
    values: &std::collections::BTreeMap<String, String>,
) -> BehaviorResult<()> {
    let defaults = Behaviors::properties::<Settings>(&std::collections::BTreeMap::new())?;
    assert_eq!(defaults.get("limit"), Some(&BehaviorValue::Number(20.0)));
    let bound = Behaviors::properties::<Settings>(values)?;
    assert_eq!(bound.get("title"), Some(&BehaviorValue::from("カタログ")));
    assert_eq!(bound.get("enabled"), Some(&BehaviorValue::Bool(true)));
    assert_eq!(bound.get("limit"), Some(&BehaviorValue::Number(42.0)));
    assert_eq!(bound.get("ratio"), Some(&BehaviorValue::Number(0.1)));
    Ok(())
}

fn check_rejected_property_values(values: &std::collections::BTreeMap<String, String>) {
    for (key, value) in [
        ("YON_APP_LIMIT", "1.0"),
        ("YON_APP_LIMIT", "1e1"),
        ("YON_APP_LIMIT", "+1"),
        ("YON_APP_LIMIT", " 1"),
        ("YON_APP_LIMIT", "01"),
        ("YON_APP_LIMIT", "-0"),
        ("YON_APP_LIMIT", "101"),
        ("YON_APP_LIMIT", "9007199254740992"),
        ("YON_APP_RATIO", "NaN"),
        ("YON_APP_RATIO", ".1"),
        ("YON_APP_RATIO", "1."),
        ("YON_APP_RATIO", "+0.1"),
        ("YON_APP_RATIO", "Infinity"),
        ("YON_APP_RATIO", "1e999"),
        ("YON_APP_ENABLED", "TRUE"),
        ("YON_APP_ENABLED", "1"),
        ("YON_APP_UNKNOWN", "redacted-secret"),
        ("TAC_APP_TITLE", "redacted-secret"),
        ("YON_bad", "redacted-secret"),
    ] {
        let mut data = values.clone();
        data.insert(key.into(), value.into());
        let error = Behaviors::properties::<Settings>(&data).unwrap_err();
        assert_eq!(error.code, "configuration-properties");
        assert!(!error.message.contains(value) || value.len() < 5);
    }
}

fn check_property_size_limits() {
    let long_title =
        std::collections::BTreeMap::from([(String::from("YON_APP_TITLE"), "é".repeat(2049))]);
    let too_many_keys = (0..257)
        .map(|index| (format!("YON_VALUE_{index}"), String::new()))
        .collect();
    let too_many_bytes = (0..20)
        .map(|index| (format!("YON_VALUE_{index}"), "x".repeat(4096)))
        .collect();
    for data in [long_title, too_many_keys, too_many_bytes] {
        assert!(Behaviors::properties::<Settings>(&data).is_err());
    }
}

fn check_invalid_property_policies() {
    let no_values = std::collections::BTreeMap::new();
    let mut required = Settings::behavior_properties();
    required.fields[0].default = None;
    assert!(behavior_bind_properties(required, &no_values).is_err());
    let mut invalid = Settings::behavior_properties();
    invalid.fields[0].default = Some(true.into());
    assert!(behavior_bind_properties(invalid, &no_values).is_err());
    let mut invalid = Settings::behavior_properties();
    invalid.fields[2].min = Some(f64::NAN);
    assert!(behavior_bind_properties(invalid, &no_values).is_err());
    let mut invalid = Settings::behavior_properties();
    invalid.prefix = "TAC_APP_";
    assert!(behavior_bind_properties(invalid, &no_values).is_err());
}

fn create_conditional(
    profiles: Vec<String>,
    enabled: Option<&str>,
) -> BehaviorResult<BehaviorContext> {
    let mut options = BehaviorOptions::new("explicit-config", BehaviorSignal::new());
    options.profiles = profiles;
    if let Some(enabled) = enabled {
        options
            .properties
            .insert("YON_FEATURE_ENABLED".into(), enabled.into());
    }
    Behaviors::create::<ConditionalConfig>(options)
}

fn check_conditional_configuration() -> BehaviorResult<()> {
    for (profiles, enabled) in [
        (vec![], Some("true")),
        (vec![String::from("test")], None),
        (vec![String::from("test")], Some("false")),
        (vec![String::from("production")], Some("true")),
    ] {
        let error = create_conditional(profiles, enabled).err().unwrap();
        assert_eq!(error.code, "inactive-configuration");
    }
    assert_eq!(ConditionalConfig::configure_call_count(), 0);
    for profile in ["development", "test"] {
        create_conditional(vec![profile.into()], Some("true"))?.dispose();
    }
    assert_eq!(ConditionalConfig::configure_call_count(), 2);
    for profiles in [
        vec![String::from("test"), String::from("test")],
        vec![String::from("invalid-profile")],
        vec![String::from("é")],
        vec!["x".repeat(65)],
        (0..33).map(|index| format!("profile{index}")).collect(),
    ] {
        let error = create_conditional(profiles, Some("true")).err().unwrap();
        assert_eq!(error.code, "configuration");
    }
    assert_eq!(ConditionalConfig::configure_call_count(), 2);
    check_shared_setup()
}

fn create_shared() -> BehaviorResult<BehaviorContext> {
    create_conditional(vec![String::from("development")], Some("true"))
}

fn check_shared_setup() -> BehaviorResult<()> {
    let configured_before = ConditionalConfig::configure_call_count();
    let writer = create_shared()?;
    let reader = create_shared()?;
    assert_eq!(ConditionalConfig::configure_call_count(), configured_before + 1);
    assert!(std::sync::Arc::ptr_eq(&writer.0.configuration, &reader.0.configuration));
    writer.dispose();
    writer.dispose();
    let later = create_shared()?;
    assert_eq!(ConditionalConfig::configure_call_count(), configured_before + 1);
    reader.dispose();
    drop(later);
    create_shared()?.dispose();
    assert_eq!(ConditionalConfig::configure_call_count(), configured_before + 2);
    Ok(())
}

fn configuration_contracts() -> BehaviorResult<()> {
    let values = property_values();
    check_property_binding(&values)?;
    check_rejected_property_values(&values);
    check_property_size_limits();
    check_invalid_property_policies();
    check_conditional_configuration()
}

struct ValidatedInput;
impl BehaviorValidated for ValidatedInput {
    fn behavior_validation() -> BehaviorValidationSchema {
        BehaviorValidationSchema {
            fields: vec![
                BehaviorValidationField {
                    name: "name",
                    not_null: true,
                    not_blank: true,
                    size: Some((1, 8)),
                    min: None,
                    max: None,
                },
                BehaviorValidationField {
                    name: "amount",
                    not_null: false,
                    not_blank: false,
                    size: None,
                    min: Some(1.0),
                    max: Some(10.0),
                },
                BehaviorValidationField {
                    name: "items",
                    not_null: false,
                    not_blank: false,
                    size: Some((1, 2)),
                    min: None,
                    max: None,
                },
            ],
        }
    }
}

fn validation_failure_path(value: BehaviorValue) -> Option<String> {
    Behaviors::validate::<ValidatedInput>(value)
        .unwrap_err()
        .path
}

fn check_validated_shape(value: &BehaviorValue) -> BehaviorResult<()> {
    assert_eq!(
        Behaviors::validate::<ValidatedInput>(value.clone())?,
        *value
    );
    assert_eq!(
        validation_failure_path(BehaviorValue::Null).as_deref(),
        Some("$.object")
    );
    assert_eq!(
        validation_failure_path(empty_object()).as_deref(),
        Some("name.notNull")
    );
    Ok(())
}

fn check_blank_names() {
    for character in [
        0x0009, 0x000a, 0x000b, 0x000c, 0x000d, 0x0020, 0x0085, 0x00a0, 0x1680, 0x2000, 0x2001,
        0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029,
        0x202f, 0x205f, 0x3000,
    ] {
        let blank = char::from_u32(character).unwrap().to_string();
        let error =
            Behaviors::validate::<ValidatedInput>(BehaviorValue::object([("name", blank.into())]))
                .unwrap_err();
        assert_eq!(error.code, "validation");
        assert_eq!(error.path.as_deref(), Some("name.notBlank"));
    }
    assert!(
        Behaviors::validate::<ValidatedInput>(BehaviorValue::object([("name", "\u{200b}".into())]))
            .is_ok(),
        "zero-width space is not Unicode White_Space"
    );
    let long_name = BehaviorValue::object([("name", "é".repeat(5).into())]);
    assert_eq!(
        validation_failure_path(long_name).as_deref(),
        Some("name.size")
    );
}

fn check_field_rules() {
    for (field, value, path) in [
        ("name", BehaviorValue::Null, "name.notNull"),
        ("name", true.into(), "name.notBlank"),
        ("amount", "5".into(), "amount.min"),
        ("amount", 0.into(), "amount.min"),
        ("amount", 11.into(), "amount.max"),
        ("items", true.into(), "items.size"),
        ("items", BehaviorValue::Array(vec![]), "items.size"),
        (
            "items",
            BehaviorValue::Array(vec![1.into(), 2.into(), 3.into()]),
            "items.size",
        ),
    ] {
        let mut data = std::collections::BTreeMap::from([(String::from("name"), "name".into())]);
        data.insert(field.into(), value);
        assert_eq!(
            validation_failure_path(BehaviorValue::Object(data)).as_deref(),
            Some(path)
        );
    }
    for item in [
        BehaviorValue::Null,
        BehaviorValue::Array(vec![1.into()]),
        BehaviorValue::object([("one", true.into()), ("two", false.into())]),
    ] {
        let input = BehaviorValue::object([
            ("name", "ok".into()),
            ("amount", BehaviorValue::Null),
            ("items", item),
        ]);
        assert!(Behaviors::validate::<ValidatedInput>(input).is_ok());
    }
}

fn check_validation_limits() {
    let secret = BehaviorValue::object([("name", "private-secret-token".into())]);
    let error = Behaviors::validate::<ValidatedInput>(secret).unwrap_err();
    assert!(!error.to_string().contains("private-secret-token"));
    assert!(error.path.as_ref().unwrap().len() <= 140);
    let not_a_number = BehaviorValue::object([
        ("name", "ok".into()),
        ("amount", BehaviorValue::Number(f64::NAN)),
    ]);
    assert_eq!(
        validation_failure_path(not_a_number).as_deref(),
        Some("$.json")
    );
    let mut nested = BehaviorValue::Null;
    for _ in 0..40 {
        nested = BehaviorValue::Array(vec![nested]);
    }
    let deep = BehaviorValue::object([("name", "ok".into()), ("nested", nested)]);
    let error = Behaviors::validate::<ValidatedInput>(deep).unwrap_err();
    assert_eq!(error.code, "validation");
}

fn check_invalid_schemas(value: BehaviorValue) {
    let mut schema = ValidatedInput::behavior_validation();
    schema.fields[0].name = "input.secret";
    let error = behavior_validate_value(schema, value.clone()).unwrap_err();
    assert_eq!(error.path.as_deref(), Some("$.schema"));
    let mut schema = ValidatedInput::behavior_validation();
    schema.fields[1].min = Some(f64::INFINITY);
    let error = behavior_validate_value(schema, value).unwrap_err();
    assert_eq!(error.path.as_deref(), Some("$.schema"));
}

fn check_collected_violations() {
    let input = BehaviorValue::object([("amount", "5".into()), ("items", true.into())]);
    let error = Behaviors::check::<ValidatedInput>(input.clone()).unwrap_err();
    assert_eq!(error.code, "validation");
    assert_eq!(error.path.as_deref(), Some("name.notNull"));
    let rules: Vec<(&str, &str)> = error
        .violations
        .iter()
        .map(|violation| (violation.field, violation.rule))
        .collect();
    assert_eq!(
        rules,
        vec![
            ("name", "Required"),
            ("name", "NotEmpty"),
            ("amount", "Min"),
            ("amount", "Max"),
            ("items", "Length"),
        ]
    );
    let first = Behaviors::validate::<ValidatedInput>(input).unwrap_err();
    assert_eq!(first.path.as_deref(), Some("name.notNull"));
    assert!(first.violations.is_empty());
    let error = Behaviors::check::<ValidatedInput>(BehaviorValue::Null).unwrap_err();
    assert_eq!(error.path.as_deref(), Some("$.object"));
    assert_eq!(
        error.violations,
        vec![BehaviorViolation {
            field: "",
            rule: "Checked",
        }]
    );
    let mut schema = ValidatedInput::behavior_validation();
    schema.fields[0].name = "input.secret";
    let error = BehaviorViolation::check_value(schema, empty_object()).unwrap_err();
    assert_eq!(error.path.as_deref(), Some("$.schema"));
    assert!(error.violations.is_empty());
    let value = BehaviorValue::object([("name", "ok".into())]);
    assert_eq!(Behaviors::check::<ValidatedInput>(value.clone()), Ok(value));
}

fn validation_contracts() -> BehaviorResult<()> {
    let value = BehaviorValue::object([("name", "é".into()), ("unknown", true.into())]);
    check_validated_shape(&value)?;
    check_blank_names();
    check_field_rules();
    check_validation_limits();
    check_invalid_schemas(value);
    check_collected_violations();
    Ok(())
}

struct Owner {
    behaviors: BehaviorContext,
}
impl BehaviorService for Owner {
    fn behavior_context(&self) -> &BehaviorContext {
        &self.behaviors
    }
    fn behavior_method(name: &str) -> Option<BehaviorMethodPolicy> {
        if name == "list" { Some(policy()) } else { None }
    }
}

struct MissingAuthorization;
impl BehaviorConfigurationSource for MissingAuthorization {
    fn behavior_configure(_: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}

fn capacity(context: &BehaviorContext, count: usize) -> BehaviorResult<BehaviorValue> {
    context.invoke(plain_policy(), vec![], |_| {
        if count == 0 {
            Ok(BehaviorValue::Null)
        } else {
            capacity(context, count - 1)
        }
    })
}

fn check_missing_authorization(denied: &BehaviorContext) -> BehaviorResult<()> {
    let error = denied
        .invoke(policy(), vec![empty_object()], |_| {
            panic!("unauthorized method executed")
        })
        .unwrap_err();
    assert_eq!(error.code, "unauthorized");
    let missing = Behaviors::create::<MissingAuthorization>(BehaviorOptions::new(
        "missing",
        BehaviorSignal::new(),
    ))?;
    let error = missing
        .invoke(policy(), vec![empty_object()], |_| {
            panic!("method executed without authorization")
        })
        .unwrap_err();
    assert_eq!(error.code, "invalid");
    Ok(())
}

fn check_eviction_owners(
    context: &BehaviorContext,
    denied: &BehaviorContext,
) -> BehaviorResult<()> {
    let outsider = Owner {
        behaviors: denied.clone(),
    };
    assert!(
        context
            .evict(&outsider, "list", vec![empty_object()])
            .is_err()
    );
    let owner = Owner {
        behaviors: context.clone(),
    };
    assert!(
        context
            .evict(&owner, "remote", vec![empty_object()])
            .is_err()
    );
    context.evict(&owner, "list", vec![empty_object()])?;
    Ok(())
}

fn check_invalid_page_arguments(context: &BehaviorContext) {
    let far_page = BehaviorValue::object([("page", 1_000_001.into())]);
    let result = context.invoke(policy(), vec![far_page], |_| {
        panic!("invalid pagination executed")
    });
    assert!(result.is_err());
    let unknown_sort =
        BehaviorValue::object([("sort", BehaviorValue::Array(vec!["unknown".into()]))]);
    let result = context.invoke(policy(), vec![unknown_sort], |_| {
        panic!("invalid sort executed")
    });
    assert!(result.is_err());
    let offset = BehaviorValue::object([("offset", 1.into())]);
    let result = context.invoke(policy(), vec![offset], |_| {
        panic!("invalid offset executed")
    });
    assert!(result.is_err());
}

fn check_value_size_limits() {
    let long_text = BehaviorValue::from("x".repeat(1_048_577));
    assert!(long_text.checked(1_048_576).is_err());
    let long_array = BehaviorValue::Array(vec![BehaviorValue::Null; 10_001]);
    assert!(long_array.checked(1_048_576).is_err());
    let mut deep = BehaviorValue::Null;
    for _ in 0..34 {
        deep = BehaviorValue::Array(vec![deep]);
    }
    assert!(deep.checked(1_048_576).is_err());
}

fn check_unsafe_http_requests() {
    for path in [
        "//evil.test/",
        "/%2e%2e/escape",
        "/%252e/escape",
        "/x?query",
        "/x\\path",
        "/%ff",
    ] {
        assert!(!behavior_safe_path(path));
    }
    let http = BehaviorHttpPolicy {
        client: "catalog",
        method: "GET",
        path: "/products",
        query: &[],
        headers: &[],
        body: None,
        path_variables: &[],
    };
    assert!(
        behavior_http_request(&http, BehaviorValue::object([("body", empty_object())])).is_err()
    );
    let cookie = BehaviorValue::object([("Cookie", "x".into())]);
    assert!(behavior_http_request(&http, BehaviorValue::object([("headers", cookie)])).is_err());
    let line_break = BehaviorValue::object([("X-Token", "x\r\ny".into())]);
    assert!(
        behavior_http_request(&http, BehaviorValue::object([("headers", line_break)])).is_err()
    );
}

fn check_repository_and_capacity(context: &BehaviorContext) -> BehaviorResult<()> {
    assert!(
        context
            .repository::<Product>()?
            .save(empty_object())
            .is_err()
    );
    assert!(
        context
            .repository::<Product>()?
            .delete(BehaviorValue::Number(1.5))
            .is_err()
    );
    assert_eq!(capacity(context, 64).unwrap_err().code, "capacity");
    assert!(
        capacity(context, 0).is_ok(),
        "capacity is released after errors"
    );
    Ok(())
}

fn check_timeout() -> BehaviorResult<()> {
    let mut options = BehaviorOptions::new("timeout", BehaviorSignal::new());
    options.timeout_ms = 1;
    let timed = Behaviors::create::<Config>(options)?;
    let error = timed
        .invoke(plain_policy(), vec![], |_| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            Ok(BehaviorValue::Null)
        })
        .unwrap_err();
    assert_eq!(error.code, "timeout");
    Ok(())
}

fn negative_contracts(context: &BehaviorContext) -> BehaviorResult<()> {
    let denied = create_context("denied")?;
    check_missing_authorization(&denied)?;
    check_eviction_owners(context, &denied)?;
    check_invalid_page_arguments(context);
    check_value_size_limits();
    check_unsafe_http_requests();
    check_repository_and_capacity(context)?;
    check_timeout()?;
    denied.dispose();
    let error = denied
        .invoke(plain_policy(), vec![], |_| {
            panic!("disposed method executed")
        })
        .unwrap_err();
    assert_eq!(error.code, "disposed");
    Ok(())
}

fn check_cached_invocation(context: &BehaviorContext) -> BehaviorResult<()> {
    let calls = std::cell::Cell::new(0);
    for _ in 0..2 {
        let value = context.invoke(policy(), vec![empty_object()], |args| {
            calls.set(calls.get() + 1);
            assert_eq!(args[0].get("size"), Some(&BehaviorValue::Number(20.0)));
            Ok(BehaviorValue::from("cached"))
        })?;
        assert_eq!(value, BehaviorValue::from("cached"));
    }
    assert_eq!(calls.get(), 1);
    Ok(())
}

fn check_repository(context: &BehaviorContext) -> BehaviorResult<()> {
    let repository = context.repository::<Product>()?;
    assert_eq!(
        repository.find(empty_object())?,
        BehaviorValue::from("find")
    );
    let product = BehaviorValue::object([("id", BehaviorValue::from("one"))]);
    assert_eq!(repository.save(product)?, BehaviorValue::from("save"));
    assert_eq!(
        repository.delete(BehaviorValue::from("one"))?,
        BehaviorValue::from("delete")
    );
    Ok(())
}

fn check_http_invocation(context: &BehaviorContext) -> BehaviorResult<()> {
    let mut http = plain_policy();
    http.http = Some(BehaviorHttpPolicy {
        client: "catalog",
        method: "GET",
        path: "/products",
        query: &[],
        headers: &[],
        body: None,
        path_variables: &[],
    });
    let response = context.invoke(http, vec![], |_| Ok(empty_object()))?;
    assert_eq!(response, BehaviorValue::from("transport"));
    Ok(())
}

fn check_rejected_values() {
    assert!(BehaviorValue::Number(f64::NAN).checked(1024).is_err());
    let reserved_key = BehaviorValue::object([("constructor", BehaviorValue::Null)]);
    assert!(reserved_key.checked(1024).is_err());
}

struct LedgerConfig;
impl LedgerConfig {
    fn log() -> &'static std::sync::Mutex<Vec<String>> {
        static LOG: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        &LOG
    }

    fn sequence() -> &'static std::sync::atomic::AtomicUsize {
        static SEQUENCE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        &SEQUENCE
    }

    fn record(entry: String) {
        LedgerConfig::log().lock().expect("ledger log").push(entry);
    }

    fn take() -> Vec<String> {
        std::mem::take(&mut *LedgerConfig::log().lock().expect("ledger log"))
    }
}
impl BehaviorConfigurationSource for LedgerConfig {
    fn behavior_configure(_: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        let mut config = BehaviorConfiguration::default();
        config.entities.insert("ledger".into(), std::sync::Arc::new(LedgerAdapter));
        config.entities.insert("audit".into(), std::sync::Arc::new(LedgerAdapter));
        config.authorize_entity = Some(std::sync::Arc::new(|_, _| Ok(true)));
        Ok(config)
    }
}
struct LedgerAdapter;
impl BehaviorEntityAdapter for LedgerAdapter {
    fn execute(
        &self,
        command: &BehaviorEntityCommand,
        _: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue> {
        let joined = command.transaction.as_deref().unwrap_or("none");
        LedgerConfig::record(format!("{}@{joined}", command.operation));
        let broken = BehaviorValue::from("broken");
        if command.value.as_ref().and_then(|value| value.get("id")) == Some(&broken) {
            return Err(behavior_invalid());
        }
        Ok(BehaviorValue::from(command.operation))
    }

    fn transaction(
        &self,
        command: &BehaviorTransactionCommand,
        _: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue> {
        let entry = match &command.transaction {
            Some(id) => format!("{}:{id}", command.action),
            None => command.action.to_owned(),
        };
        LedgerConfig::record(entry);
        if command.action != "begin" {
            return Ok(BehaviorValue::Null);
        }
        let number = LedgerConfig::sequence().fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        Ok(BehaviorValue::from(format!("t{number}").as_str()))
    }
}
struct Entry;
impl BehaviorEntity for Entry {
    fn behavior_entity() -> BehaviorEntityMetadata {
        BehaviorEntityMetadata {
            name: "Entry",
            adapter: "ledger",
            table: "entries",
            id: "id",
        }
    }
}
struct Note;
impl BehaviorEntity for Note {
    fn behavior_entity() -> BehaviorEntityMetadata {
        BehaviorEntityMetadata {
            name: "Note",
            adapter: "audit",
            table: "notes",
            id: "id",
        }
    }
}

fn settle(
    service: &BehaviorContext,
    store: &BehaviorContext,
    second: &str,
) -> BehaviorResult<BehaviorValue> {
    let mut policy = plain_policy();
    policy.owner = "LedgerService";
    policy.name = "settle";
    policy.transactional = Some(BehaviorTransactionPolicy { adapter: "ledger" });
    service.invoke(policy, vec![], |_| {
        store.repository::<Entry>()?.save(BehaviorValue::object([("id", "first".into())]))?;
        store.repository::<Note>()?.save(BehaviorValue::object([("id", "note".into())]))?;
        store.repository::<Entry>()?.save(BehaviorValue::object([("id", second.into())]))?;
        Ok(BehaviorValue::from("settled"))
    })
}

fn check_joined_transaction() -> BehaviorResult<()> {
    let service = Behaviors::create::<LedgerConfig>(BehaviorOptions::new("ledger", BehaviorSignal::new()))?;
    let store = Behaviors::create::<LedgerConfig>(BehaviorOptions::new("entries", BehaviorSignal::new()))?;
    assert_eq!(settle(&service, &store, "second")?, BehaviorValue::from("settled"));
    assert_eq!(
        LedgerConfig::take(),
        ["begin", "save@t1", "save@none", "save@t1", "commit:t1"]
    );
    assert!(settle(&service, &store, "broken").is_err());
    assert_eq!(
        LedgerConfig::take(),
        ["begin", "save@t2", "save@none", "save@t2", "rollback:t2"]
    );
    Ok(())
}

fn main() -> BehaviorResult<()> {
    validation_contracts()?;
    configuration_contracts()?;
    mutation_contracts()?;
    query_contracts()?;
    let signal = BehaviorSignal::new();
    let context = Behaviors::create::<Config>(BehaviorOptions::new("tenant-a", signal.clone()))?;
    check_cached_invocation(&context)?;
    check_repository(&context)?;
    check_joined_transaction()?;
    check_http_invocation(&context)?;
    check_rejected_values();
    negative_contracts(&context)?;
    signal.abort();
    let error = context
        .invoke(policy(), vec![], |_| Ok(BehaviorValue::Null))
        .unwrap_err();
    assert_eq!(error.code, "aborted");
    context.dispose();
    println!("rust behavior runtime contracts passed");
    Ok(())
}
