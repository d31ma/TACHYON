#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorError {
    pub code: &'static str,
    pub message: &'static str,
    pub path: Option<String>,
}
impl std::fmt::Display for BehaviorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)?;
        if let Some(path) = &self.path {
            write!(formatter, " ({path})")?;
        }
        Ok(())
    }
}
impl std::error::Error for BehaviorError {}
pub type BehaviorResult<T> = Result<T, BehaviorError>;
fn behavior_error(code: &'static str, message: &'static str) -> BehaviorError {
    BehaviorError {
        code,
        message,
        path: None,
    }
}
fn behavior_invalid() -> BehaviorError {
    behavior_error("invalid", "Invalid or out-of-bounds behavior data.")
}
fn behavior_retryable(error: &BehaviorError) -> bool {
    !matches!(
        error.code,
        "invalid" | "unauthorized" | "capacity" | "cancelled" | "timeout" | "configuration"
    )
}
fn behavior_backoff(retry: &BehaviorRetryPolicy, attempt: u32) -> u64 {
    let delay = (retry.delay_ms as f64 * retry.multiplier.powi(attempt as i32))
        .min(retry.max_delay_ms as f64);
    let jitter = if retry.jitter_ms == 0 {
        0.0
    } else {
        (behavior_now().unwrap_or(0) % (retry.jitter_ms + 1)) as f64
    };
    (delay + jitter).round().max(0.0) as u64
}

#[derive(Clone, Debug, PartialEq)]
pub enum BehaviorValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<BehaviorValue>),
    Object(std::collections::BTreeMap<String, BehaviorValue>),
}
impl BehaviorValue {
    pub fn object<K: Into<String>>(entries: impl IntoIterator<Item = (K, Self)>) -> Self {
        let mut values = std::collections::BTreeMap::new();
        values.extend(entries.into_iter().map(|(key, value)| (key.into(), value)));
        Self::Object(values)
    }
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(values) => values.get(key),
            _ => None,
        }
    }
    pub fn map_value(
        &self,
        plan: &[(&str, Option<&[&str]>, Self)],
    ) -> BehaviorResult<Self> {
        if !matches!(self, Self::Object(_)) {
            return Err(behavior_invalid());
        }
        let mut output = Vec::new();
        for (target, source, constant) in plan {
            let Some(path) = source else {
                output.push(((*target).to_owned(), constant.clone()));
                continue;
            };
            let mut current = Some(self);
            for name in *path {
                current = current.and_then(|value| value.get(name));
            }
            output.push(((*target).to_owned(), current.cloned().unwrap_or(Self::Null)));
        }
        Self::object(output).checked(1_048_576)
    }
    pub fn checked(&self, limit: usize) -> BehaviorResult<Self> {
        self.to_json(limit)?;
        Ok(self.clone())
    }
    pub fn to_json(&self, limit: usize) -> BehaviorResult<String> {
        if limit == 0 || limit > 1_048_576 {
            return Err(behavior_invalid());
        }
        let mut writer = BehaviorJsonWriter {
            output: String::new(),
            nodes: 0,
            limit,
        };
        writer.value(self, 0)?;
        Ok(writer.output)
    }
}
impl From<&str> for BehaviorValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}
impl From<String> for BehaviorValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<bool> for BehaviorValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<i32> for BehaviorValue {
    fn from(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
}
struct BehaviorJsonWriter {
    output: String,
    nodes: usize,
    limit: usize,
}
impl BehaviorJsonWriter {
    fn append(&mut self, value: &str) -> BehaviorResult<()> {
        if value.len() > self.limit.saturating_sub(self.output.len()) {
            return Err(behavior_invalid());
        }
        self.output.push_str(value);
        Ok(())
    }
    fn string(&mut self, value: &str) -> BehaviorResult<()> {
        self.append("\"")?;
        for character in value.chars() {
            match character {
                '"' => self.append("\\\"")?,
                '\\' => self.append("\\\\")?,
                '\n' => self.append("\\n")?,
                '\r' => self.append("\\r")?,
                '\t' => self.append("\\t")?,
                value if value < ' ' => self.append(&format!("\\u{:04x}", u32::from(value)))?,
                value => self.append(value.encode_utf8(&mut [0; 4]))?,
            }
        }
        self.append("\"")
    }
    fn value(&mut self, value: &BehaviorValue, depth: usize) -> BehaviorResult<()> {
        self.nodes += 1;
        if self.nodes > 10_000 || depth > 32 {
            return Err(behavior_invalid());
        }
        match value {
            BehaviorValue::Null => self.append("null"),
            BehaviorValue::Bool(value) => self.append(if *value { "true" } else { "false" }),
            BehaviorValue::Number(value) if value.is_finite() => self.append(&value.to_string()),
            BehaviorValue::Number(_) => Err(behavior_invalid()),
            BehaviorValue::String(value) => self.string(value),
            BehaviorValue::Array(values) => self.array(values, depth),
            BehaviorValue::Object(values) => self.object(values, depth),
        }
    }
    fn array(&mut self, values: &[BehaviorValue], depth: usize) -> BehaviorResult<()> {
        if values.len() > 10_000 {
            return Err(behavior_invalid());
        }
        self.append("[")?;
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                self.append(",")?;
            }
            self.value(value, depth + 1)?;
        }
        self.append("]")
    }
    fn object(
        &mut self,
        values: &std::collections::BTreeMap<String, BehaviorValue>,
        depth: usize,
    ) -> BehaviorResult<()> {
        if values.len() > 10_000 {
            return Err(behavior_invalid());
        }
        self.append("{")?;
        for (index, (key, value)) in values.iter().enumerate() {
            let reserved = matches!(
                key.as_str(),
                "__proto__" | "prototype" | "constructor" | "behaviors"
            );
            if reserved || (key == "tac" && depth != 0) {
                return Err(behavior_invalid());
            }
            if index > 0 {
                self.append(",")?;
            }
            self.string(key)?;
            self.append(":")?;
            self.value(value, depth + 1)?;
        }
        self.append("}")
    }
}

#[derive(Clone, Default, Debug)]
pub struct BehaviorSignal(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl BehaviorSignal {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn abort(&self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn aborted(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}
#[derive(Clone)]
pub struct BehaviorCancellation {
    owner: BehaviorSignal,
    disposed: BehaviorSignal,
    deadline: u64,
}
impl BehaviorCancellation {
    pub fn aborted(&self) -> bool {
        self.owner.aborted() || self.disposed.aborted() || behavior_monotonic_ms() >= self.deadline
    }
}
#[derive(Clone)]
pub struct BehaviorExecution {
    pub environment: &'static str,
    pub partition: String,
    pub signal: BehaviorCancellation,
    pub deadline: u64,
    monotonic_deadline: u64,
    disposed: BehaviorSignal,
}
impl BehaviorExecution {
    pub fn check(&self) -> BehaviorResult<()> {
        if self.disposed.aborted() {
            return Err(behavior_error("disposed", "Behavior context is disposed."));
        }
        if self.signal.owner.aborted() {
            return Err(behavior_error("aborted", "Behavior operation was aborted."));
        }
        if behavior_monotonic_ms() >= self.monotonic_deadline {
            return Err(behavior_error("timeout", "Behavior operation timed out."));
        }
        Ok(())
    }
}
pub struct BehaviorOptions {
    pub partition: String,
    pub signal: BehaviorSignal,
    pub timeout_ms: u64,
    pub profiles: Vec<String>,
    pub properties: std::collections::BTreeMap<String, String>,
}
impl BehaviorOptions {
    pub fn new(partition: impl Into<String>, signal: BehaviorSignal) -> Self {
        Self {
            partition: partition.into(),
            signal,
            timeout_ms: 10_000,
            profiles: Vec::new(),
            properties: std::collections::BTreeMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct BehaviorCachePolicy {
    pub name: &'static str,
    pub ttl_ms: u64,
    pub max_bytes: usize,
    pub max_entries: usize,
}
#[derive(Clone)]
pub struct BehaviorPagePolicy {
    pub argument: usize,
    pub default_size: u64,
    pub max_size: u64,
    pub sort: Vec<&'static str>,
}
#[derive(Clone)]
pub struct BehaviorHttpPolicy {
    pub query: &'static [(&'static str, usize)],
    pub headers: &'static [(&'static str, usize)],
    pub body: Option<usize>,
    pub client: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub path_variables: &'static [(&'static str, usize)],
}
#[derive(Clone)]
pub struct BehaviorCacheMutationPolicy {
    pub operation: &'static str,
    pub method: &'static str,
    pub arguments: Vec<usize>,
    pub key_args: Vec<usize>,
    pub pageable: Option<BehaviorPagePolicy>,
}
#[derive(Clone)]
pub struct BehaviorMethodPolicy {
    pub owner: &'static str,
    pub name: &'static str,
    pub cache: Option<BehaviorCachePolicy>,
    pub key_args: Option<Vec<usize>>,
    pub pageable: Option<BehaviorPagePolicy>,
    pub http: Option<BehaviorHttpPolicy>,
    pub mutation: Option<BehaviorCacheMutationPolicy>,
    pub retry: Option<BehaviorRetryPolicy>,
    pub limit: Option<BehaviorLimitPolicy>,
    pub transactional: Option<BehaviorTransactionPolicy>,
}
#[derive(Clone)]
pub struct BehaviorRetryPolicy {
    pub max_retries: u32,
    pub delay_ms: u64,
    pub multiplier: f64,
    pub max_delay_ms: u64,
    pub jitter_ms: u64,
}
#[derive(Clone)]
pub struct BehaviorLimitPolicy {
    pub limit: usize,
    pub blocking: bool,
}
#[derive(Clone)]
pub struct BehaviorTransactionPolicy {
    pub adapter: &'static str,
}
#[derive(Clone)]
pub struct BehaviorCacheEntry {
    pub value: BehaviorValue,
    pub expires_at: u64,
}
pub struct BehaviorCacheWrite {
    pub execution: BehaviorExecution,
    pub policy: BehaviorCachePolicy,
    pub expires_at: u64,
}
pub trait BehaviorCacheAdapter: Send + Sync {
    fn get(
        &self,
        key: &str,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<Option<BehaviorCacheEntry>>;
    fn set(
        &self,
        key: &str,
        value: BehaviorValue,
        write: &BehaviorCacheWrite,
    ) -> BehaviorResult<()>;
    fn delete(&self, key: &str, execution: &BehaviorExecution) -> BehaviorResult<()>;
}
pub struct BehaviorAuthorizationCall {
    pub operation: &'static str,
    pub cache: &'static str,
    pub owner: &'static str,
    pub method: &'static str,
    pub key_arguments: Vec<BehaviorValue>,
}
pub struct BehaviorHttpRequest {
    pub method: &'static str,
    pub path: String,
    pub query: BehaviorValue,
    pub headers: BehaviorValue,
    pub body: Option<BehaviorValue>,
    pub redirect: &'static str,
    pub credentials: &'static str,
    pub max_response_bytes: usize,
}
pub trait BehaviorHttpAdapter: Send + Sync {
    fn exchange(
        &self,
        request: &BehaviorHttpRequest,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue>;
}
#[derive(Clone)]
pub struct BehaviorEntityMetadata {
    pub name: &'static str,
    pub adapter: &'static str,
    pub table: &'static str,
    pub id: &'static str,
}
#[derive(Clone)]
pub struct BehaviorQueryPredicate {
    pub field: &'static str,
    pub operator: &'static str,
    pub argument: Option<usize>,
}
#[derive(Clone)]
pub struct BehaviorQuerySort {
    pub field: &'static str,
    pub direction: &'static str,
}
#[derive(Clone)]
pub struct BehaviorQueryPolicy {
    pub entity: BehaviorEntityMetadata,
    pub fields: Vec<&'static str>,
    pub default_size: u64,
    pub max_size: u64,
    pub operation: &'static str,
    pub predicates: Vec<BehaviorQueryPredicate>,
    pub order_by: Vec<BehaviorQuerySort>,
    pub page_argument: Option<usize>,
}
#[derive(Clone)]
pub struct BehaviorEntityCommand {
    pub entity: &'static str,
    pub table: &'static str,
    pub id_field: &'static str,
    pub operation: &'static str,
    pub value: Option<BehaviorValue>,
    pub page: Option<BehaviorValue>,
    pub id: Option<BehaviorValue>,
    pub query: Option<BehaviorValue>,
    pub transaction: Option<String>,
}

pub struct BehaviorTransactionCommand {
    pub action: &'static str,
    pub transaction: Option<String>,
}
pub trait BehaviorEntityAdapter: Send + Sync {
    fn execute(
        &self,
        command: &BehaviorEntityCommand,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue>;

    fn transaction(
        &self,
        command: &BehaviorTransactionCommand,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<BehaviorValue> {
        let _ = (command, execution);
        Err(behavior_error(
            "adapter",
            "This entity adapter has no transactions.",
        ))
    }
}
pub trait BehaviorEntity {
    fn behavior_entity() -> BehaviorEntityMetadata;
}
pub trait BehaviorService {
    fn behavior_context(&self) -> &BehaviorContext;
    fn behavior_method(name: &str) -> Option<BehaviorMethodPolicy>;
}
pub type BehaviorAuthorize =
    dyn Fn(&BehaviorAuthorizationCall, &BehaviorExecution) -> BehaviorResult<bool> + Send + Sync;
pub type BehaviorAuthorizeEntity =
    dyn Fn(&BehaviorEntityCommand, &BehaviorExecution) -> BehaviorResult<bool> + Send + Sync;
#[derive(Default)]
pub struct BehaviorConfiguration {
    pub caches: std::collections::BTreeMap<String, std::sync::Arc<dyn BehaviorCacheAdapter>>,
    pub http: std::collections::BTreeMap<String, std::sync::Arc<dyn BehaviorHttpAdapter>>,
    pub entities: std::collections::BTreeMap<String, std::sync::Arc<dyn BehaviorEntityAdapter>>,
    pub authorize: Option<std::sync::Arc<BehaviorAuthorize>>,
    pub authorize_entity: Option<std::sync::Arc<BehaviorAuthorizeEntity>>,
}
pub trait BehaviorConfigurationSource {
    fn behavior_configure(execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration>;
    fn behavior_conditions() -> BehaviorConfigurationConditions {
        BehaviorConfigurationConditions::default()
    }
}
#[derive(Default)]
pub struct BehaviorConfigurationConditions {
    pub profiles: Vec<&'static str>,
    pub property: Option<BehaviorPropertyCondition>,
}
pub struct BehaviorPropertyCondition {
    pub name: &'static str,
    pub having_value: &'static str,
    pub match_if_missing: bool,
}
pub struct BehaviorPropertiesPolicy {
    pub prefix: &'static str,
    pub fields: Vec<BehaviorPropertyField>,
}
pub struct BehaviorPropertyField {
    pub field: &'static str,
    pub name: &'static str,
    pub kind: &'static str,
    pub default: Option<BehaviorValue>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}
pub trait BehaviorPropertiesSource {
    fn behavior_properties() -> BehaviorPropertiesPolicy;
}
pub struct BehaviorValuesPolicy {
    pub fields: Vec<BehaviorPropertyField>,
}
pub trait BehaviorValuesSource {
    fn behavior_values() -> BehaviorValuesPolicy;
}
pub struct BehaviorValidationSchema {
    pub fields: Vec<BehaviorValidationField>,
}
pub struct BehaviorValidationField {
    pub name: &'static str,
    pub not_null: bool,
    pub not_blank: bool,
    pub size: Option<(usize, usize)>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}
pub trait BehaviorValidated {
    fn behavior_validation() -> BehaviorValidationSchema;
}
struct BehaviorOpenTransactions {
    outer: Vec<(&'static str, String)>,
}
impl BehaviorOpenTransactions {
    fn with<R>(action: impl FnOnce(&std::cell::RefCell<Vec<(&'static str, String)>>) -> R) -> R {
        thread_local! {
            static OPEN: std::cell::RefCell<Vec<(&'static str, String)>> = const { std::cell::RefCell::new(Vec::new()) };
        }
        OPEN.with(action)
    }
    fn of(adapter: &str) -> Option<String> {
        Self::with(|open| {
            open.borrow()
                .iter()
                .find(|(name, _)| *name == adapter)
                .map(|(_, id)| id.clone())
        })
    }
    fn open(adapter: &'static str, id: String) -> Self {
        let outer = Self::with(|open| open.borrow().clone());
        Self::with(|open| open.borrow_mut().push((adapter, id)));
        Self { outer }
    }
}
impl Drop for BehaviorOpenTransactions {
    fn drop(&mut self) {
        let outer = std::mem::take(&mut self.outer);
        Self::with(|open| *open.borrow_mut() = outer);
    }
}
struct BehaviorContextState {
    options: BehaviorOptions,
    configuration: std::sync::Arc<BehaviorConfiguration>,
    setup_key: String,
    released: std::sync::atomic::AtomicBool,
    disposed: BehaviorSignal,
    active: std::sync::atomic::AtomicUsize,
    slots: std::sync::Mutex<std::collections::BTreeMap<(&'static str, &'static str), usize>>,
    idle: std::sync::Condvar,
    revalidate: std::sync::Mutex<std::collections::BTreeSet<String>>,
}
impl BehaviorContextState {
    fn dispose(&self) {
        self.disposed.abort();
        if !self.released.swap(true, std::sync::atomic::Ordering::SeqCst) {
            Behaviors::release_setup(&self.setup_key);
        }
    }
}
impl Drop for BehaviorContextState {
    fn drop(&mut self) {
        self.dispose();
    }
}
struct BehaviorSharedSetup {
    configuration: std::sync::Arc<BehaviorConfiguration>,
    users: usize,
}
#[derive(Clone)]
pub struct BehaviorContext(std::sync::Arc<BehaviorContextState>);
struct BehaviorOperation<'a> {
    context: &'a BehaviorContext,
    execution: BehaviorExecution,
}
impl Drop for BehaviorOperation<'_> {
    fn drop(&mut self) {
        self.context
            .0
            .active
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}
impl BehaviorOperation<'_> {
    fn adapter<T>(&self, call: impl FnOnce() -> BehaviorResult<T>) -> BehaviorResult<T> {
        self.execution.check()?;
        let value = call().map_err(|error| {
            if error.code == "storage" {
                error
            } else {
                behavior_error("adapter", "Behavior adapter failed.")
            }
        });
        self.execution.check()?;
        value
    }
}
pub struct Behaviors;
impl Behaviors {
    fn setups() -> std::sync::MutexGuard<'static, std::collections::BTreeMap<String, BehaviorSharedSetup>> {
        static ENTRIES: std::sync::Mutex<std::collections::BTreeMap<String, BehaviorSharedSetup>> =
            std::sync::Mutex::new(std::collections::BTreeMap::new());
        ENTRIES.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn setup_key<C: BehaviorConfigurationSource>(options: &BehaviorOptions) -> String {
        let mut profiles = options.profiles.clone();
        profiles.sort();
        format!("{}{profiles:?}{:?}", std::any::type_name::<C>(), options.properties)
    }
    fn acquire_setup<C: BehaviorConfigurationSource>(
        key: &str,
        execution: &BehaviorExecution,
    ) -> BehaviorResult<std::sync::Arc<BehaviorConfiguration>> {
        let mut entries = Self::setups();
        if let Some(shared) = entries.get_mut(key) {
            shared.users += 1;
            return Ok(shared.configuration.clone());
        }
        let configuration = std::sync::Arc::new(behavior_configuration::<C>(execution)?);
        let shared = BehaviorSharedSetup { configuration: configuration.clone(), users: 1 };
        entries.insert(key.to_owned(), shared);
        Ok(configuration)
    }
    fn release_setup(key: &str) {
        let mut entries = Self::setups();
        let Some(shared) = entries.get_mut(key) else {
            return;
        };
        shared.users -= 1;
        if shared.users == 0 {
            entries.remove(key);
        }
    }
    pub fn validate<V: BehaviorValidated>(value: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        behavior_validate_value(V::behavior_validation(), value)
    }
    pub fn properties<P: BehaviorPropertiesSource>(
        values: &impl BehaviorPropertyInput,
    ) -> BehaviorResult<BehaviorValue> {
        values
            .behavior_property_inputs()
            .and_then(|values| behavior_bind_properties(P::behavior_properties(), &values))
            .map_err(|_| {
                behavior_error(
                    "configuration-properties",
                    "Invalid or missing explicit configuration properties.",
                )
            })
    }
    pub fn values<V: BehaviorValuesSource>(
        application: &BehaviorValue,
    ) -> BehaviorResult<BehaviorValue> {
        behavior_bind_values(V::behavior_values(), application).map_err(|_| {
            behavior_error(
                "configuration-properties",
                "Invalid or missing application configuration value.",
            )
        })
    }
    pub fn create<C: BehaviorConfigurationSource>(
        options: BehaviorOptions,
    ) -> BehaviorResult<BehaviorContext> {
        if options.partition.trim().is_empty()
            || options.partition.len() > 256
            || !(1..=60_000).contains(&options.timeout_ms)
        {
            return Err(behavior_invalid());
        }
        behavior_validate_properties(&options.properties)
            .and_then(|_| behavior_validate_profiles(options.profiles.iter().map(String::as_str)))
            .map_err(|_| {
                behavior_error("configuration", "Invalid explicit configuration inputs.")
            })?;
        let disposed = BehaviorSignal::new();
        let execution = behavior_execution(&options, &disposed)?;
        execution.check()?;
        behavior_active_configuration::<C>(&options)?;
        let setup_key = Self::setup_key::<C>(&options);
        let configuration = match Self::acquire_setup::<C>(&setup_key, &execution) {
            Ok(configuration) => configuration,
            Err(error) => {
                disposed.abort();
                return Err(error);
            }
        };
        Ok(BehaviorContext(std::sync::Arc::new(BehaviorContextState {
            options,
            configuration,
            setup_key,
            released: std::sync::atomic::AtomicBool::new(false),
            disposed,
            active: std::sync::atomic::AtomicUsize::new(0),
            slots: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            idle: std::sync::Condvar::new(),
            revalidate: std::sync::Mutex::new(std::collections::BTreeSet::new()),
        })))
    }
    pub fn gateway_arguments(arguments: Vec<BehaviorValue>) -> BehaviorResult<String> {
        BehaviorValue::Array(arguments)
            .to_json(1_048_576)
            .map_err(|_| Self::gateway_failure("invalid"))
    }
    pub fn gateway_result(answer: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let BehaviorValue::Object(mut fields) = answer else {
            return Err(Self::gateway_failure("adapter"));
        };
        if let Some(result) = fields.remove("result") {
            return Ok(result);
        }
        match fields.get("error") {
            Some(BehaviorValue::String(kind)) => Err(Self::gateway_failure(kind)),
            _ => Err(Self::gateway_failure("adapter")),
        }
    }
    fn gateway_failure(kind: &str) -> BehaviorError {
        let code = match kind {
            "invalid" => "invalid",
            "timeout" => "timeout",
            "capacity" => "capacity",
            _ => "adapter",
        };
        behavior_error(code, "Gateway call failed.")
    }
}
fn behavior_validation_error(field: &str, rule: &str) -> BehaviorError {
    BehaviorError {
        code: "validation",
        message: "Behavior validation failed.",
        path: Some(format!("{field}.{rule}")),
    }
}
fn behavior_unicode_whitespace(character: char) -> bool {
    matches!(u32::from(character), 0x0009..=0x000d | 0x0020 | 0x0085 | 0x00a0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000)
}
fn behavior_validate_value(
    schema: BehaviorValidationSchema,
    value: BehaviorValue,
) -> BehaviorResult<BehaviorValue> {
    let value = value
        .checked(1_048_576)
        .map_err(|_| behavior_validation_error("$", "json"))?;
    let BehaviorValue::Object(values) = &value else {
        return Err(behavior_validation_error("$", "object"));
    };
    if schema.fields.len() > 64 {
        return Err(behavior_validation_error("$", "schema"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for field in schema.fields {
        if !behavior_identifier(field.name)
            || !seen.insert(field.name)
            || behavior_invalid_validation_bounds(&field)
        {
            return Err(behavior_validation_error("$", "schema"));
        }
        behavior_validate_field(&field, values.get(field.name))?;
    }
    Ok(value)
}
fn behavior_invalid_validation_bounds(field: &BehaviorValidationField) -> bool {
    let invalid_size = field
        .size
        .is_some_and(|(min, max)| min > max || max > 1_048_576);
    let invalid_number = field
        .min
        .into_iter()
        .chain(field.max)
        .any(|value| !behavior_property_number(value, false));
    let reversed = field.min.zip(field.max).is_some_and(|(min, max)| min > max);
    invalid_size || invalid_number || reversed
}
fn behavior_validate_field(
    field: &BehaviorValidationField,
    item: Option<&BehaviorValue>,
) -> BehaviorResult<()> {
    let absent = matches!(item, None | Some(BehaviorValue::Null));
    if field.not_null && absent {
        return Err(behavior_validation_error(field.name, "notNull"));
    }
    if field.not_blank && !behavior_not_blank(item) {
        return Err(behavior_validation_error(field.name, "notBlank"));
    }
    if absent {
        return Ok(());
    }
    if let Some((min, max)) = field.size {
        let size = match item {
            Some(BehaviorValue::String(value)) => value.len(),
            Some(BehaviorValue::Array(value)) => value.len(),
            Some(BehaviorValue::Object(value)) => value.len(),
            _ => return Err(behavior_validation_error(field.name, "size")),
        };
        if !(min..=max).contains(&size) {
            return Err(behavior_validation_error(field.name, "size"));
        }
    }
    for (rule, bound) in [("min", field.min), ("max", field.max)] {
        let Some(bound) = bound else {
            continue;
        };
        let Some(BehaviorValue::Number(number)) = item else {
            return Err(behavior_validation_error(field.name, rule));
        };
        if (rule == "min" && *number < bound) || (rule == "max" && *number > bound) {
            return Err(behavior_validation_error(field.name, rule));
        }
    }
    Ok(())
}
fn behavior_not_blank(item: Option<&BehaviorValue>) -> bool {
    let Some(BehaviorValue::String(text)) = item else {
        return false;
    };
    !text.is_empty() && !text.chars().all(behavior_unicode_whitespace)
}

fn behavior_framework_prefix() -> &'static str {
    if __TACHYON_BEHAVIOR_ENVIRONMENT == "tac" {
        "TAC_"
    } else {
        "YON_"
    }
}
fn behavior_environment_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.as_bytes()[0].is_ascii_uppercase()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}
fn behavior_validate_properties(
    values: &std::collections::BTreeMap<String, String>,
) -> BehaviorResult<()> {
    if values.len() > 256
        || values.iter().any(|(name, value)| {
            !behavior_environment_name(name)
                || !name.starts_with(behavior_framework_prefix())
                || value.len() > 4096
        })
    {
        return Err(behavior_invalid());
    }
    BehaviorValue::object(
        values
            .iter()
            .map(|(name, value)| (name.clone(), BehaviorValue::from(value.clone()))),
    )
    .to_json(65_536)?;
    Ok(())
}
fn behavior_validate_profiles<'a>(
    profiles: impl IntoIterator<Item = &'a str>,
) -> BehaviorResult<()> {
    let mut unique = std::collections::BTreeSet::new();
    for profile in profiles {
        if profile.is_empty()
            || profile.len() > 64
            || !unique.insert(profile)
            || !profile.bytes().enumerate().all(|(index, byte)| {
                byte.is_ascii_alphabetic()
                    || matches!(byte, b'_' | b'$')
                    || (index > 0 && byte.is_ascii_digit())
            })
            || unique.len() > 32
        {
            return Err(behavior_invalid());
        }
    }
    Ok(())
}
fn behavior_active_configuration<C: BehaviorConfigurationSource>(
    options: &BehaviorOptions,
) -> BehaviorResult<()> {
    let conditions = C::behavior_conditions();
    behavior_validate_profiles(conditions.profiles.iter().copied()).map_err(|_| {
        behavior_error(
            "configuration",
            "Invalid explicit configuration conditions.",
        )
    })?;
    let profiles_match = conditions.profiles.is_empty()
        || conditions
            .profiles
            .iter()
            .any(|profile| options.profiles.iter().any(|active| active == profile));
    let property_match = match conditions.property {
        None => true,
        Some(condition) => behavior_property_condition_matches(&condition, options)?,
    };
    if !profiles_match || !property_match {
        return Err(behavior_error(
            "inactive-configuration",
            "Setup is inactive for the explicit selection.",
        ));
    }
    Ok(())
}
fn behavior_property_condition_matches(
    condition: &BehaviorPropertyCondition,
    options: &BehaviorOptions,
) -> BehaviorResult<bool> {
    if !behavior_environment_name(condition.name)
        || !condition.name.starts_with(behavior_framework_prefix())
        || condition.having_value.len() > 4096
    {
        return Err(behavior_error(
            "configuration",
            "Invalid explicit configuration conditions.",
        ));
    }
    let nested_prefix = format!("{}_", condition.name);
    if options
        .properties
        .keys()
        .any(|key| key.starts_with(&nested_prefix))
    {
        return Ok(false);
    }
    let matches = match options.properties.get(condition.name) {
        Some(value) => value == condition.having_value,
        None => condition.match_if_missing,
    };
    Ok(matches)
}
fn behavior_property_number(value: f64, integer: bool) -> bool {
    value.is_finite()
        && value.abs() <= 9_007_199_254_740_991.0
        && (!integer || value.fract() == 0.0)
}
fn behavior_number_token(value: &str, integer: bool) -> bool {
    let bytes = value.as_bytes();
    let mut position = usize::from(bytes.first() == Some(&b'-'));
    match bytes.get(position) {
        Some(b'0') => position += 1,
        Some(b'1'..=b'9') => position = behavior_skip_digits(bytes, position + 1),
        _ => return false,
    }
    if !integer && bytes.get(position) == Some(&b'.') {
        let first = position + 1;
        position = behavior_skip_digits(bytes, first);
        if first == position {
            return false;
        }
    }
    if !integer && matches!(bytes.get(position), Some(b'e' | b'E')) {
        position += 1;
        if matches!(bytes.get(position), Some(b'+' | b'-')) {
            position += 1;
        }
        let first = position;
        position = behavior_skip_digits(bytes, first);
        if first == position {
            return false;
        }
    }
    position == bytes.len()
}
fn behavior_skip_digits(bytes: &[u8], mut position: usize) -> usize {
    while bytes.get(position).is_some_and(u8::is_ascii_digit) {
        position += 1;
    }
    position
}
fn behavior_property_value(
    field: &BehaviorPropertyField,
    raw: Option<&String>,
) -> BehaviorResult<BehaviorValue> {
    let numeric = matches!(field.kind, "integer" | "number");
    let integer = field.kind == "integer";
    if !matches!(field.kind, "string" | "boolean" | "integer" | "number")
        || field
            .min
            .into_iter()
            .chain(field.max)
            .any(|value| !numeric || !behavior_property_number(value, integer))
        || field.min.zip(field.max).is_some_and(|(min, max)| min > max)
    {
        return Err(behavior_invalid());
    }
    let value = match raw {
        Some(raw) => behavior_parse_property(field.kind, raw, integer)?,
        None => field.default.clone().ok_or_else(behavior_invalid)?,
    };
    if !behavior_property_value_allowed(field, &value) {
        return Err(behavior_invalid());
    }
    Ok(value)
}
fn behavior_parse_property(kind: &str, raw: &str, integer: bool) -> BehaviorResult<BehaviorValue> {
    match kind {
        "string" => Ok(raw.into()),
        "boolean" => match raw {
            "true" => Ok(true.into()),
            "false" => Ok(false.into()),
            _ => Err(behavior_invalid()),
        },
        _ if behavior_number_token(raw, integer) => raw
            .parse::<f64>()
            .map(BehaviorValue::Number)
            .map_err(|_| behavior_invalid()),
        _ => Err(behavior_invalid()),
    }
}
fn behavior_property_value_allowed(field: &BehaviorPropertyField, value: &BehaviorValue) -> bool {
    let integer = field.kind == "integer";
    match value {
        BehaviorValue::String(text) => field.kind == "string" && text.len() <= 4096,
        BehaviorValue::Bool(_) => field.kind == "boolean",
        BehaviorValue::Number(number) => {
            matches!(field.kind, "integer" | "number")
                && behavior_property_number(*number, integer)
                && !field.min.is_some_and(|min| *number < min)
                && !field.max.is_some_and(|max| *number > max)
        }
        _ => false,
    }
}
fn behavior_bind_properties(
    policy: BehaviorPropertiesPolicy,
    values: &std::collections::BTreeMap<String, String>,
) -> BehaviorResult<BehaviorValue> {
    behavior_validate_properties(values)?;
    if !behavior_environment_name(policy.prefix)
        || policy.prefix.len() > 128
        || !policy.prefix.starts_with(behavior_framework_prefix())
        || !policy.prefix.ends_with('_')
        || policy.fields.is_empty()
        || policy.fields.len() > 64
    {
        return Err(behavior_invalid());
    }
    let mut names = std::collections::BTreeSet::new();
    let mut output = std::collections::BTreeMap::new();
    for field in &policy.fields {
        let name = format!("{}{}", policy.prefix, field.name);
        if !behavior_identifier(field.field)
            || !behavior_environment_name(field.name)
            || name.len() > 256
            || !names.insert(name.clone())
            || output.contains_key(field.field)
        {
            return Err(behavior_invalid());
        }
        let value = behavior_property_value(field, values.get(&name))?;
        output.insert(field.field.to_owned(), value);
    }
    if values
        .keys()
        .any(|name| name.starts_with(policy.prefix) && !names.contains(name))
    {
        return Err(behavior_invalid());
    }
    BehaviorValue::Object(output).checked(65_536)
}

fn behavior_application_inputs(
    application: &BehaviorValue,
) -> BehaviorResult<std::collections::BTreeMap<String, String>> {
    application.checked(65_536)?;
    let BehaviorValue::Object(root) = application else {
        return Err(behavior_invalid());
    };
    let mut flat = std::collections::BTreeMap::new();
    if !root.iter().any(|(key, _)| matches!(key.as_str(), "tac" | "yon")) {
        return Err(behavior_invalid());
    }
    for (key, value) in root {
        if !matches!(key.as_str(), "tac" | "yon") {
            continue;
        }
        if !matches!(value, BehaviorValue::Object(_)) {
            return Err(behavior_invalid());
        }
        behavior_flatten_application(value, key, 0, &mut flat)?;
    }
    Ok(flat)
}
fn behavior_flatten_application(
    value: &BehaviorValue,
    path: &str,
    depth: usize,
    flat: &mut std::collections::BTreeMap<String, String>,
) -> BehaviorResult<()> {
    if depth > 16 || path.len() > 256 || flat.len() >= 256 {
        return Err(behavior_invalid());
    }
    if path == "yon.server.port"
        && !matches!(value, BehaviorValue::Number(number) if behavior_property_number(*number, true))
    {
        return Err(behavior_invalid());
    }
    let BehaviorValue::Object(values) = value else {
        return behavior_flatten_scalar(value, path, flat);
    };
    if depth > 0 && values.is_empty() {
        flat.insert(format!("{path}."), String::new());
    }
    for (key, value) in values {
        if !behavior_application_key(key) {
            return Err(behavior_invalid());
        }
        behavior_flatten_application(value, &format!("{path}.{key}"), depth + 1, flat)?;
    }
    Ok(())
}
fn behavior_application_key(key: &str) -> bool {
    !key.is_empty()
        && key.as_bytes()[0].is_ascii_lowercase()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}
fn behavior_flatten_scalar(
    value: &BehaviorValue,
    path: &str,
    flat: &mut std::collections::BTreeMap<String, String>,
) -> BehaviorResult<()> {
    let text = match value {
        BehaviorValue::Null | BehaviorValue::Array(_) => {
            flat.insert(format!("{path}."), String::new());
            return Ok(());
        }
        BehaviorValue::String(value) => value.clone(),
        BehaviorValue::Bool(value) => value.to_string(),
        BehaviorValue::Number(value) if value.is_finite() => value.to_string(),
        _ => return Err(behavior_invalid()),
    };
    if text.len() > 4096 || flat.len() >= 256 {
        return Err(behavior_invalid());
    }
    flat.insert(path.to_owned(), text);
    Ok(())
}
pub trait BehaviorPropertyInput {
    fn behavior_property_inputs(
        &self,
    ) -> BehaviorResult<std::collections::BTreeMap<String, String>>;
}
impl BehaviorPropertyInput for std::collections::BTreeMap<String, String> {
    fn behavior_property_inputs(
        &self,
    ) -> BehaviorResult<std::collections::BTreeMap<String, String>> {
        behavior_validate_properties(self)?;
        Ok(self.clone())
    }
}
impl BehaviorPropertyInput for BehaviorValue {
    fn behavior_property_inputs(
        &self,
    ) -> BehaviorResult<std::collections::BTreeMap<String, String>> {
        let prefix = format!("{}.", __TACHYON_BEHAVIOR_ENVIRONMENT);
        let mut inputs = std::collections::BTreeMap::new();
        inputs.extend(
            behavior_application_inputs(self)?
                .into_iter()
                .filter(|(path, _)| path.starts_with(&prefix))
                .map(|(path, value)| (path.replace('.', "_").to_ascii_uppercase(), value)),
        );
        Ok(inputs)
    }
}
impl BehaviorOptions {
    pub fn with_application(mut self, application: &BehaviorValue) -> BehaviorResult<Self> {
        self.properties = application.behavior_property_inputs()?;
        Ok(self)
    }
}
fn behavior_bind_values(
    policy: BehaviorValuesPolicy,
    application: &BehaviorValue,
) -> BehaviorResult<BehaviorValue> {
    if policy.fields.is_empty() || policy.fields.len() > 64 {
        return Err(behavior_invalid());
    }
    let values = behavior_application_inputs(application)?;
    let mut output = std::collections::BTreeMap::new();
    for field in &policy.fields {
        if !behavior_identifier(field.field) || output.contains_key(field.field) {
            return Err(behavior_invalid());
        }
        if values
            .keys()
            .any(|path| path.starts_with(&format!("{}.", field.name)))
        {
            return Err(behavior_invalid());
        }
        let value = behavior_property_value(field, values.get(field.name))?;
        output.insert(field.field.to_owned(), value);
    }
    BehaviorValue::Object(output).checked(65_536)
}

fn behavior_configuration<C: BehaviorConfigurationSource>(
    execution: &BehaviorExecution,
) -> BehaviorResult<BehaviorConfiguration> {
    let configuration = C::behavior_configure(execution)
        .map_err(|_| behavior_error("adapter", "Behavior configuration failed."));
    execution.check()?;
    let configuration = configuration?;
    behavior_adapter_names(configuration.caches.keys())?;
    behavior_adapter_names(configuration.http.keys())?;
    behavior_adapter_names(configuration.entities.keys())?;
    Ok(configuration)
}
fn behavior_adapter_names<'a>(
    mut keys: impl ExactSizeIterator<Item = &'a String>,
) -> BehaviorResult<()> {
    if keys.len() > 128 || keys.any(|key| !behavior_identifier(key)) {
        return Err(behavior_invalid());
    }
    Ok(())
}
fn behavior_execution(
    options: &BehaviorOptions,
    disposed: &BehaviorSignal,
) -> BehaviorResult<BehaviorExecution> {
    let monotonic_deadline = behavior_monotonic_ms()
        .checked_add(options.timeout_ms)
        .ok_or_else(behavior_invalid)?;
    Ok(BehaviorExecution {
        environment: __TACHYON_BEHAVIOR_ENVIRONMENT,
        partition: options.partition.clone(),
        signal: BehaviorCancellation {
            owner: options.signal.clone(),
            disposed: disposed.clone(),
            deadline: monotonic_deadline,
        },
        deadline: behavior_now()?
            .checked_add(options.timeout_ms)
            .ok_or_else(behavior_invalid)?,
        monotonic_deadline,
        disposed: disposed.clone(),
    })
}
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "tachyon")]
unsafe extern "C" {
    fn tac_host_monotonic_ms() -> f64;
    fn tac_host_epoch_ms() -> f64;
}
#[cfg(not(target_arch = "wasm32"))]
fn behavior_monotonic_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    u64::try_from(START.get_or_init(std::time::Instant::now).elapsed().as_millis()).unwrap_or(u64::MAX)
}
#[cfg(target_arch = "wasm32")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn behavior_monotonic_ms() -> u64 {
    unsafe { tac_host_monotonic_ms() }.max(0.0) as u64
}
#[cfg(not(target_arch = "wasm32"))]
fn behavior_now() -> BehaviorResult<u64> {
    let value = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| behavior_invalid())?
        .as_millis();
    u64::try_from(value).map_err(|_| behavior_invalid())
}
#[cfg(target_arch = "wasm32")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn behavior_now() -> BehaviorResult<u64> {
    Ok(unsafe { tac_host_epoch_ms() }.max(0.0) as u64)
}
fn behavior_sleep(milliseconds: u64) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::sleep(std::time::Duration::from_millis(milliseconds));
    #[cfg(target_arch = "wasm32")]
    {
        let until = behavior_monotonic_ms().saturating_add(milliseconds);
        while behavior_monotonic_ms() < until {}
    }
}
fn behavior_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte == b'_'
                || byte == b'$'
                || byte.is_ascii_alphabetic()
                || (index > 0 && byte.is_ascii_digit())
        })
        && !matches!(
            value,
            "__proto__" | "prototype" | "constructor" | "behaviors" | "tac"
        )
}
impl BehaviorContext {
    pub fn rest_client(&self, name: &'static str) -> BehaviorResult<BehaviorRestClient> {
        if !behavior_identifier(name) {
            return Err(behavior_invalid());
        }
        Ok(BehaviorRestClient {
            context: self.clone(),
            name,
        })
    }

    fn cache_adapter(&self, name: &str) -> BehaviorResult<std::sync::Arc<dyn BehaviorCacheAdapter>> {
        self.0
            .configuration
            .caches
            .get(name)
            .cloned()
            .or_else(|| behavior_builtin_cache(name))
            .ok_or_else(behavior_invalid)
    }
    fn builtin_cache(&self, name: &str) -> bool {
        !self.0.configuration.caches.contains_key(name) && behavior_builtin_cache(name).is_some()
    }
    fn builtin_entities(&self, name: &str) -> bool {
        !self.0.configuration.entities.contains_key(name)
            && behavior_builtin_entities(name).is_some()
    }
    fn http_adapter(&self, name: &str) -> BehaviorResult<&std::sync::Arc<dyn BehaviorHttpAdapter>> {
        self.0.configuration.http.get(name).ok_or_else(behavior_invalid)
    }
    fn entity_adapter(
        &self,
        name: &str,
    ) -> BehaviorResult<std::sync::Arc<dyn BehaviorEntityAdapter>> {
        self.0
            .configuration
            .entities
            .get(name)
            .cloned()
            .or_else(|| behavior_builtin_entities(name))
            .ok_or_else(behavior_invalid)
    }

    fn rest_exchange(
        &self,
        name: &'static str,
        method: &'static str,
        path: &str,
        descriptor: BehaviorValue,
    ) -> BehaviorResult<BehaviorValue> {
        let operation = self.operation()?;
        if !behavior_safe_path(path) {
            return Err(behavior_invalid());
        }
        let policy = BehaviorHttpPolicy {
            client: name,
            method,
            path: "/",
            query: &[],
            headers: &[],
            body: None,
            path_variables: &[],
        };
        let mut request = behavior_http_request(&policy, descriptor)?;
        request.path = path.to_owned();
        let adapter = self.http_adapter(name)?;
        operation
            .adapter(|| adapter.exchange(&request, &operation.execution))?
            .checked(1_048_576)
    }

    pub fn query(
        &self,
        policy: BehaviorQueryPolicy,
        args: Vec<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let active = self.operation()?;
        let query = behavior_query(&policy, &args)?;
        let builtin = self.0.configuration.authorize_entity.is_none()
            && self.builtin_entities(policy.entity.adapter);
        let authorize = self.0.configuration.authorize_entity.clone();
        if authorize.is_none() && !builtin {
            return Err(behavior_invalid());
        }
        let adapter = self.entity_adapter(policy.entity.adapter)?;
        let command = BehaviorEntityCommand {
            transaction: BehaviorOpenTransactions::of(policy.entity.adapter),
            operation: "query",
            entity: policy.entity.name,
            table: policy.entity.table,
            id_field: policy.entity.id,
            query: Some(query),
            value: None,
            page: None,
            id: None,
        };
        behavior_check_query_command(&command)?;
        if let Some(authorize) = authorize
            && !active.adapter(|| authorize(&command, &active.execution))?
        {
            return Err(behavior_error(
                "unauthorized",
                "Store query was not authorized.",
            ));
        }
        let result = active
            .adapter(|| adapter.execute(&command, &active.execution))?
            .checked(1_048_576)?;
        behavior_check_query_result(policy.operation, &command, &result)?;
        active.execution.check()?;
        match (policy.operation, result) {
            ("findOne", BehaviorValue::Array(rows)) => {
                Ok(rows.into_iter().next().unwrap_or(BehaviorValue::Null))
            }
            (_, result) => Ok(result),
        }
    }
    pub fn dispose(&self) {
        self.0.dispose();
    }
    fn operation(&self) -> BehaviorResult<BehaviorOperation<'_>> {
        let execution = behavior_execution(&self.0.options, &self.0.disposed)?;
        execution.check()?;
        self.0
            .active
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |active| (active < 64).then_some(active + 1),
            )
            .map_err(|_| {
                behavior_error("capacity", "Behavior context concurrency bound reached.")
            })?;
        Ok(BehaviorOperation {
            context: self,
            execution,
        })
    }
    pub fn invoke(
        &self,
        policy: BehaviorMethodPolicy,
        input: Vec<BehaviorValue>,
        original: impl Fn(Vec<BehaviorValue>) -> BehaviorResult<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let operation = self.operation()?;
        let args = behavior_arguments(input, &policy)?;
        let value = if let Some(mutation) = &policy.mutation {
            self.mutate(&operation, &policy, mutation, args, original)?
        } else if let Some(indices) = &policy.key_args {
            return self.cached(&operation, &policy, indices, args, original);
        } else {
            self.result(&operation, &policy, args, original)?
        };
        operation.execution.check()?;
        Ok(value)
    }
    fn cached(
        &self,
        operation: &BehaviorOperation<'_>,
        policy: &BehaviorMethodPolicy,
        indices: &[usize],
        args: Vec<BehaviorValue>,
        original: impl Fn(Vec<BehaviorValue>) -> BehaviorResult<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let cache = policy.cache.as_ref().ok_or_else(behavior_invalid)?;
        behavior_validate_cache(cache)?;
        if behavior_http_method(policy.name) {
            return Err(behavior_invalid());
        }
        let selected = behavior_selected(indices, &args)?;
        self.authorize(operation, policy, selected.clone(), "cache:read")?;
        let key = self.cache_key(policy, selected)?;
        let adapter = self.cache_adapter(cache.name)?;
        let entry = operation.adapter(|| adapter.get(&key, &operation.execution))?;
        if let Some(value) = behavior_fresh_entry(entry, cache)? {
            operation.execution.check()?;
            return Ok(value);
        }
        let value = self
            .result(operation, policy, args, original)?
            .checked(cache.max_bytes)?;
        let write = BehaviorCacheWrite {
            execution: operation.execution.clone(),
            policy: cache.clone(),
            expires_at: behavior_now()?.saturating_add(cache.ttl_ms),
        };
        operation.adapter(|| adapter.set(&key, value.clone(), &write))?;
        operation.execution.check()?;
        Ok(value)
    }
    fn mutate(
        &self,
        operation: &BehaviorOperation<'_>,
        policy: &BehaviorMethodPolicy,
        mutation: &BehaviorCacheMutationPolicy,
        args: Vec<BehaviorValue>,
        original: impl Fn(Vec<BehaviorValue>) -> BehaviorResult<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        if policy.key_args.is_some()
            || !matches!(mutation.operation, "cache:put" | "cache:evict")
            || !behavior_identifier(mutation.method)
            || behavior_http_method(policy.name)
            || mutation.arguments.len() > 32
        {
            return Err(behavior_invalid());
        }
        let cache = policy.cache.as_ref().ok_or_else(behavior_invalid)?;
        behavior_validate_cache(cache)?;
        let target = behavior_mutation_target(policy, mutation);
        let mapped = behavior_mutation_arguments(mutation, &args)?;
        let target_args = behavior_arguments(mapped, &target)?;
        let selected = behavior_selected(&mutation.key_args, &target_args)?;
        self.authorize(operation, &target, selected.clone(), mutation.operation)?;
        let key = self.cache_key(&target, selected)?;
        let adapter = self.cache_adapter(cache.name)?;
        let limit = if mutation.operation == "cache:put" {
            cache.max_bytes
        } else {
            1_048_576
        };
        let value = self
            .result(operation, policy, args, original)?
            .checked(limit)?;
        operation.execution.check()?;
        if mutation.operation == "cache:put" {
            let write = BehaviorCacheWrite {
                execution: operation.execution.clone(),
                policy: cache.clone(),
                expires_at: behavior_now()?
                    .checked_add(cache.ttl_ms)
                    .ok_or_else(behavior_invalid)?,
            };
            operation.adapter(|| adapter.set(&key, value.clone(), &write))?;
        } else {
            operation.adapter(|| adapter.delete(&key, &operation.execution))?;
        }
        Ok(value)
    }
    fn result(
        &self,
        operation: &BehaviorOperation<'_>,
        policy: &BehaviorMethodPolicy,
        args: Vec<BehaviorValue>,
        original: impl Fn(Vec<BehaviorValue>) -> BehaviorResult<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let Some(limit) = &policy.limit else {
            return self.attempt(operation, policy, args, original);
        };
        let key = (policy.owner, policy.name);
        self.acquire_slot(operation, key, limit)?;
        let value = self.attempt(operation, policy, args, original);
        self.release_slot(key);
        value
    }
    fn acquire_slot(
        &self,
        operation: &BehaviorOperation<'_>,
        key: (&'static str, &'static str),
        limit: &BehaviorLimitPolicy,
    ) -> BehaviorResult<()> {
        let mut slots = self.0.slots.lock().map_err(|_| behavior_invalid())?;
        while slots.get(&key).copied().unwrap_or(0) >= limit.limit {
            if !limit.blocking || cfg!(target_arch = "wasm32") {
                return Err(behavior_error(
                    "capacity",
                    "Behavior method concurrency limit reached.",
                ));
            }
            operation.execution.check()?;
            let (guard, _) = self
                .0
                .idle
                .wait_timeout(slots, std::time::Duration::from_millis(50))
                .map_err(|_| behavior_invalid())?;
            slots = guard;
        }
        *slots.entry(key).or_insert(0) += 1;
        Ok(())
    }
    fn release_slot(&self, key: (&'static str, &'static str)) {
        if let Ok(mut slots) = self.0.slots.lock() {
            let taken = slots.entry(key).or_insert(1);
            *taken = taken.saturating_sub(1);
            if *taken == 0 {
                slots.remove(&key);
            }
        }
        self.0.idle.notify_one();
    }
    fn transacted(
        &self,
        operation: &BehaviorOperation<'_>,
        policy: &BehaviorMethodPolicy,
        args: Vec<BehaviorValue>,
        original: &impl Fn(Vec<BehaviorValue>) -> BehaviorResult<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let Some(transactional) = policy
            .transactional
            .as_ref()
            .filter(|transactional| BehaviorOpenTransactions::of(transactional.adapter).is_none())
        else {
            return self.execute(operation, policy, args, original);
        };
        let adapter = self.entity_adapter(transactional.adapter)?;
        let control = |action: &'static str, id: Option<String>| {
            let command = BehaviorTransactionCommand {
                action,
                transaction: id,
            };
            operation.adapter(|| adapter.transaction(&command, &operation.execution))
        };
        let begun = control("begin", None)?;
        let id = match begun {
            BehaviorValue::String(id) if !id.is_empty() && id.len() <= 128 => id,
            _ => return Err(behavior_invalid()),
        };
        let open = BehaviorOpenTransactions::open(transactional.adapter, id.clone());
        let value = self.execute(operation, policy, args, original);
        drop(open);
        if value.is_err() {
            let _ = control("rollback", Some(id));
            return value;
        }
        control("commit", Some(id))?;
        value
    }
    fn attempt(
        &self,
        operation: &BehaviorOperation<'_>,
        policy: &BehaviorMethodPolicy,
        args: Vec<BehaviorValue>,
        original: impl Fn(Vec<BehaviorValue>) -> BehaviorResult<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let Some(retry) = &policy.retry else {
            return self.transacted(operation, policy, args, &original);
        };
        for attempt in 0..=retry.max_retries {
            match self.transacted(operation, policy, args.clone(), &original) {
                Ok(value) => return Ok(value),
                Err(error) if attempt == retry.max_retries || !behavior_retryable(&error) => {
                    return Err(error);
                }
                Err(_) => {
                    operation.execution.check()?;
                    let wait = behavior_backoff(retry, attempt);
                    if wait > 0 {
                        behavior_sleep(wait);
                    }
                    operation.execution.check()?;
                }
            }
        }
        Err(behavior_invalid())
    }
    fn execute(
        &self,
        operation: &BehaviorOperation<'_>,
        policy: &BehaviorMethodPolicy,
        args: Vec<BehaviorValue>,
        original: &impl Fn(Vec<BehaviorValue>) -> BehaviorResult<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        operation.execution.check()?;
        let filled_path = match &policy.http {
            Some(http) => Some(behavior_exchange_path(http, &args)?),
            None => None,
        };
        let declared_exchange = policy.http.as_ref().filter(|http| {
            !http.query.is_empty()
                || !http.headers.is_empty()
                || http.body.is_some()
                || !http.path_variables.is_empty()
        });
        let value = match declared_exchange {
            Some(http) => Ok(behavior_exchange_request(http, &args)?),
            None => original(args),
        };
        operation.execution.check()?;
        let value = value?;
        let Some(http) = &policy.http else {
            return value.checked(1_048_576);
        };
        let mut request = behavior_http_request(http, value)?;
        if let Some(path) = filled_path {
            request.path = path;
        }
        let adapter = self.http_adapter(http.client)?;
        operation
            .adapter(|| adapter.exchange(&request, &operation.execution))?
            .checked(1_048_576)
    }
    fn authorize(
        &self,
        operation: &BehaviorOperation<'_>,
        policy: &BehaviorMethodPolicy,
        selected: Vec<BehaviorValue>,
        intent: &'static str,
    ) -> BehaviorResult<()> {
        let cache = policy.cache.as_ref().ok_or_else(behavior_invalid)?;
        if self.0.configuration.authorize.is_none() && self.builtin_cache(cache.name) {
            return Ok(());
        }
        let authorize = self
            .0
            .configuration
            .authorize
            .as_ref()
            .ok_or_else(behavior_invalid)?;
        let call = BehaviorAuthorizationCall {
            operation: intent,
            cache: cache.name,
            owner: policy.owner,
            method: policy.name,
            key_arguments: selected,
        };
        if !operation.adapter(|| authorize(&call, &operation.execution))? {
            return Err(behavior_error(
                "unauthorized",
                "Behavior cache access was not authorized.",
            ));
        }
        Ok(())
    }
    fn cache_key(
        &self,
        policy: &BehaviorMethodPolicy,
        selected: Vec<BehaviorValue>,
    ) -> BehaviorResult<String> {
        let cache = policy.cache.as_ref().ok_or_else(behavior_invalid)?;
        BehaviorValue::Array(vec![
            self.0.options.partition.clone().into(),
            cache.name.into(),
            __TACHYON_BEHAVIOR_MODULE_ID.into(),
            policy.owner.into(),
            policy.name.into(),
            BehaviorValue::Array(selected),
        ])
        .to_json(4096)
    }
    pub fn evict<S: BehaviorService>(
        &self,
        service: &S,
        method: &str,
        input: Vec<BehaviorValue>,
    ) -> BehaviorResult<()> {
        if !std::sync::Arc::ptr_eq(&self.0, &service.behavior_context().0) {
            return Err(behavior_invalid());
        }
        let policy = S::behavior_method(method).ok_or_else(behavior_invalid)?;
        let indices = policy.key_args.as_ref().ok_or_else(behavior_invalid)?;
        let operation = self.operation()?;
        let args = behavior_arguments(input, &policy)?;
        let selected = behavior_selected(indices, &args)?;
        self.authorize(&operation, &policy, selected.clone(), "cache:evict")?;
        let key = self.cache_key(&policy, selected)?;
        let cache = policy.cache.as_ref().ok_or_else(behavior_invalid)?;
        let adapter = self.cache_adapter(cache.name)?;
        operation.adapter(|| adapter.delete(&key, &operation.execution))
    }
    pub fn repository<E: BehaviorEntity>(&self) -> BehaviorResult<BehaviorRepository> {
        let metadata = E::behavior_entity();
        if [metadata.name, metadata.adapter, metadata.table, metadata.id]
            .iter()
            .any(|name| !behavior_identifier(name))
        {
            return Err(behavior_invalid());
        }
        Ok(BehaviorRepository {
            context: self.clone(),
            metadata,
        })
    }
}
fn behavior_check_query_command(command: &BehaviorEntityCommand) -> BehaviorResult<()> {
    BehaviorValue::object([
        ("operation", command.operation.into()),
        ("entity", command.entity.into()),
        ("table", command.table.into()),
        ("idField", command.id_field.into()),
        ("query", command.query.clone().ok_or_else(behavior_invalid)?),
    ])
    .to_json(1_048_576)?;
    Ok(())
}
fn behavior_check_query_result(
    operation: &str,
    command: &BehaviorEntityCommand,
    result: &BehaviorValue,
) -> BehaviorResult<()> {
    match operation {
        "find" | "findOne" => {
            let size = command
                .query
                .as_ref()
                .and_then(|query| query.get("page"))
                .and_then(|page| page.get("size"));
            let size = behavior_integer(size, 0, 1, 100)?;
            let BehaviorValue::Array(rows) = result else {
                return Err(behavior_invalid());
            };
            let all_objects = rows.iter().all(|row| matches!(row, BehaviorValue::Object(_)));
            if rows.len() as u64 > size || !all_objects {
                return Err(behavior_invalid());
            }
        }
        "count" | "delete" => {
            behavior_integer(Some(result), 0, 0, 9_007_199_254_740_991)?;
        }
        "exists" => {
            if !matches!(result, BehaviorValue::Bool(_)) {
                return Err(behavior_invalid());
            }
        }
        _ => return Err(behavior_invalid()),
    }
    Ok(())
}
fn behavior_http_method(name: &str) -> bool {
    matches!(
        name,
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    )
}
fn behavior_fresh_entry(
    entry: Option<BehaviorCacheEntry>,
    cache: &BehaviorCachePolicy,
) -> BehaviorResult<Option<BehaviorValue>> {
    let Some(entry) = entry else {
        return Ok(None);
    };
    let now = behavior_now()?;
    if entry.expires_at > now.saturating_add(cache.ttl_ms) {
        return Err(behavior_invalid());
    }
    if entry.expires_at <= now {
        return Ok(None);
    }
    Ok(Some(entry.value.checked(cache.max_bytes)?))
}
fn behavior_mutation_target(
    policy: &BehaviorMethodPolicy,
    mutation: &BehaviorCacheMutationPolicy,
) -> BehaviorMethodPolicy {
    let mut target = policy.clone();
    target.name = mutation.method;
    target.key_args = Some(mutation.key_args.clone());
    target.pageable = mutation.pageable.clone();
    target.mutation = None;
    target.http = None;
    target
}
fn behavior_mutation_arguments(
    mutation: &BehaviorCacheMutationPolicy,
    args: &[BehaviorValue],
) -> BehaviorResult<Vec<BehaviorValue>> {
    let mut mapped = Vec::new();
    for index in &mutation.arguments {
        if *index >= 32 {
            return Err(behavior_invalid());
        }
        mapped.push(args.get(*index).cloned().ok_or_else(behavior_invalid)?);
    }
    Ok(mapped)
}
fn behavior_validate_cache(policy: &BehaviorCachePolicy) -> BehaviorResult<()> {
    if !behavior_identifier(policy.name)
        || !(1..=86_400_000).contains(&policy.ttl_ms)
        || !(1..=1_048_576).contains(&policy.max_bytes)
        || !(1..=10_000).contains(&policy.max_entries)
    {
        return Err(behavior_invalid());
    }
    Ok(())
}
fn behavior_selected(
    indices: &[usize],
    input: &[BehaviorValue],
) -> BehaviorResult<Vec<BehaviorValue>> {
    let mut seen = std::collections::BTreeSet::new();
    if indices.len() > 32
        || indices
            .iter()
            .any(|index| *index >= 32 || !seen.insert(*index))
    {
        return Err(behavior_invalid());
    }
    let values = indices
        .iter()
        .map(|index| {
            input
                .get(*index)
                .ok_or_else(behavior_invalid)?
                .checked(4096)
        })
        .collect::<BehaviorResult<Vec<_>>>()?;
    BehaviorValue::Array(values.clone()).to_json(4096)?;
    Ok(values)
}
fn behavior_arguments(
    mut input: Vec<BehaviorValue>,
    policy: &BehaviorMethodPolicy,
) -> BehaviorResult<Vec<BehaviorValue>> {
    if input.len() > 64 {
        return Err(behavior_invalid());
    }
    if let Some(page) = &policy.pageable {
        let value = input.get_mut(page.argument).ok_or_else(behavior_invalid)?;
        *value = behavior_page(value, page)?;
    }
    Ok(input)
}
fn behavior_integer(
    value: Option<&BehaviorValue>,
    fallback: u64,
    minimum: u64,
    maximum: u64,
) -> BehaviorResult<u64> {
    let number = match value {
        None => return Ok(fallback),
        Some(BehaviorValue::Number(value)) => *value,
        _ => return Err(behavior_invalid()),
    };
    if !number.is_finite()
        || number.fract() != 0.0
        || number < minimum as f64
        || number > maximum as f64
    {
        return Err(behavior_invalid());
    }
    Ok(number as u64)
}
fn behavior_page(
    input: &BehaviorValue,
    policy: &BehaviorPagePolicy,
) -> BehaviorResult<BehaviorValue> {
    if behavior_invalid_page_policy(policy) {
        return Err(behavior_invalid());
    }
    input.to_json(4096)?;
    let BehaviorValue::Object(values) = input else {
        return Err(behavior_invalid());
    };
    if values
        .keys()
        .any(|key| !["page", "size", "sort", "offset"].contains(&key.as_str()))
    {
        return Err(behavior_invalid());
    }
    let page = behavior_integer(values.get("page"), 0, 0, 1_000_000)?;
    let size = behavior_integer(values.get("size"), policy.default_size, 1, policy.max_size)?;
    let offset = page
        .checked_mul(size)
        .filter(|offset| *offset <= 1_000_000)
        .ok_or_else(behavior_invalid)?;
    if behavior_integer(values.get("offset"), offset, 0, 1_000_000)? != offset {
        return Err(behavior_invalid());
    }
    let normalized = behavior_page_sort(values.get("sort"), policy)?;
    Ok(BehaviorValue::object([
        ("page", BehaviorValue::Number(page as f64)),
        ("size", BehaviorValue::Number(size as f64)),
        ("offset", BehaviorValue::Number(offset as f64)),
        ("sort", BehaviorValue::Array(normalized)),
    ]))
}
fn behavior_invalid_page_policy(policy: &BehaviorPagePolicy) -> bool {
    policy.argument >= 32
        || policy.default_size == 0
        || policy.default_size > policy.max_size
        || policy.max_size > 1000
        || policy.sort.len() > 32
        || policy.sort.iter().any(|name| !behavior_identifier(name))
}
fn behavior_page_sort(
    sort: Option<&BehaviorValue>,
    policy: &BehaviorPagePolicy,
) -> BehaviorResult<Vec<BehaviorValue>> {
    let entries = match sort {
        None => return Ok(Vec::new()),
        Some(BehaviorValue::Array(entries)) => entries,
        _ => return Err(behavior_invalid()),
    };
    if entries.len() > 16 {
        return Err(behavior_invalid());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut normalized = Vec::new();
    for entry in entries {
        let BehaviorValue::String(entry) = entry else {
            return Err(behavior_invalid());
        };
        let parts = entry.split(':').collect::<Vec<_>>();
        let field = parts[0];
        let direction = parts.get(1).copied().unwrap_or("asc");
        if parts.len() > 2
            || !policy.sort.contains(&field)
            || !["asc", "desc"].contains(&direction)
            || !seen.insert(field.to_owned())
        {
            return Err(behavior_invalid());
        }
        normalized.push(BehaviorValue::from(format!("{field}:{direction}")));
    }
    Ok(normalized)
}
pub struct BehaviorRestClient {
    context: BehaviorContext,
    name: &'static str,
}
impl BehaviorRestClient {
    pub fn get(&self, path: &str) -> BehaviorRestCall {
        self.call("GET", path)
    }
    pub fn post(&self, path: &str) -> BehaviorRestCall {
        self.call("POST", path)
    }
    pub fn put(&self, path: &str) -> BehaviorRestCall {
        self.call("PUT", path)
    }
    pub fn patch(&self, path: &str) -> BehaviorRestCall {
        self.call("PATCH", path)
    }
    pub fn delete(&self, path: &str) -> BehaviorRestCall {
        self.call("DELETE", path)
    }
    pub fn head(&self, path: &str) -> BehaviorRestCall {
        self.call("HEAD", path)
    }
    pub fn options(&self, path: &str) -> BehaviorRestCall {
        self.call("OPTIONS", path)
    }
    fn call(&self, method: &'static str, path: &str) -> BehaviorRestCall {
        BehaviorRestCall {
            context: self.context.clone(),
            name: self.name,
            method,
            path: path.to_owned(),
            query: std::collections::BTreeMap::new(),
            headers: std::collections::BTreeMap::new(),
            body: None,
        }
    }
}

pub struct BehaviorRestCall {
    context: BehaviorContext,
    name: &'static str,
    method: &'static str,
    path: String,
    query: std::collections::BTreeMap<String, BehaviorValue>,
    headers: std::collections::BTreeMap<String, BehaviorValue>,
    body: Option<BehaviorValue>,
}
impl BehaviorRestCall {
    #[must_use]
    pub fn query(mut self, name: &str, value: BehaviorValue) -> Self {
        self.query.insert(name.to_owned(), value);
        self
    }
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers
            .insert(name.to_owned(), BehaviorValue::from(value));
        self
    }
    #[must_use]
    pub fn body(mut self, value: BehaviorValue) -> Self {
        self.body = Some(value);
        self
    }
    pub fn retrieve(self) -> BehaviorResult<BehaviorValue> {
        if self.body.is_some() && matches!(self.method, "GET" | "HEAD") {
            return Err(behavior_invalid());
        }
        let mut descriptor = vec![
            ("query", BehaviorValue::Object(self.query.clone())),
            ("headers", BehaviorValue::Object(self.headers.clone())),
        ];
        if let Some(body) = self.body.clone() {
            descriptor.push(("body", body));
        }
        self.context.rest_exchange(
            self.name,
            self.method,
            &self.path,
            BehaviorValue::object(descriptor),
        )
    }
}

pub struct BehaviorRepository {
    context: BehaviorContext,
    metadata: BehaviorEntityMetadata,
}
impl BehaviorRepository {
    fn execute(
        &self,
        operation: &'static str,
        value: Option<BehaviorValue>,
        page: Option<BehaviorValue>,
        id: Option<BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        let active = self.context.operation()?;
        let adapter = self.context.entity_adapter(self.metadata.adapter)?;
        let command = BehaviorEntityCommand {
            transaction: BehaviorOpenTransactions::of(self.metadata.adapter),
            entity: self.metadata.name,
            table: self.metadata.table,
            id_field: self.metadata.id,
            operation,
            value,
            page,
            id,
            query: None,
        };
        active
            .adapter(|| adapter.execute(&command, &active.execution))?
            .checked(1_048_576)
    }
    pub fn find(&self, page: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let page = behavior_page(
            &page,
            &BehaviorPagePolicy {
                argument: 0,
                default_size: 20,
                max_size: 100,
                sort: vec![self.metadata.id],
            },
        )?;
        self.execute("find", None, Some(page), None)
    }
    pub fn save(&self, value: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        let value = value.checked(1_048_576)?;
        behavior_entity_id(value.get(self.metadata.id).ok_or_else(behavior_invalid)?)?;
        self.execute("save", Some(value), None, None)
    }
    pub fn delete(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        behavior_entity_id(&id)?;
        self.execute("delete", None, None, Some(id))
    }
}
fn behavior_query(
    policy: &BehaviorQueryPolicy,
    args: &[BehaviorValue],
) -> BehaviorResult<BehaviorValue> {
    if behavior_invalid_query_policy(policy, args.len()) {
        return Err(behavior_invalid());
    }
    let mut used = std::collections::BTreeSet::new();
    let mut predicates = Vec::new();
    for predicate in &policy.predicates {
        predicates.push(behavior_query_predicate(policy, predicate, args, &mut used)?);
    }
    let predicates = BehaviorValue::Array(predicates).checked(65_536)?;
    let order = behavior_query_order(policy)?;
    let mut query = std::collections::BTreeMap::new();
    query.insert(String::from("kind"), policy.operation.into());
    query.insert(String::from("where"), predicates);
    query.insert(String::from("orderBy"), BehaviorValue::Array(order));
    if policy.operation == "find" {
        let page = behavior_query_page(policy, args, &mut used)?;
        query.insert(String::from("page"), page);
    }
    if policy.operation == "findOne" {
        let one = BehaviorValue::object([("size", BehaviorValue::Number(1.0))]);
        let page = behavior_page(
            &one,
            &BehaviorPagePolicy {
                argument: 0,
                default_size: 1,
                max_size: 1,
                sort: policy.fields.clone(),
            },
        )?;
        query.insert(String::from("kind"), "find".into());
        query.insert(String::from("page"), page);
    }
    if used != (0..args.len()).collect() {
        return Err(behavior_invalid());
    }
    Ok(BehaviorValue::Object(query))
}
fn behavior_invalid_query_policy(policy: &BehaviorQueryPolicy, argument_count: usize) -> bool {
    let entity = &policy.entity;
    let invalid_entity = [entity.name, entity.adapter, entity.table, entity.id]
        .iter()
        .any(|name| !behavior_identifier(name));
    let unique_fields = policy
        .fields
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let invalid_fields = policy.fields.is_empty()
        || policy.fields.len() > 64
        || policy.fields.iter().any(|field| !behavior_identifier(field))
        || unique_fields.len() != policy.fields.len()
        || !policy.fields.contains(&entity.id);
    let invalid_sizes = policy.default_size == 0
        || policy.default_size > policy.max_size
        || policy.max_size > 100;
    let invalid_limits =
        policy.predicates.len() > 8 || policy.order_by.len() > 5 || argument_count > 32;
    let invalid_operation = !matches!(
        policy.operation,
        "find" | "findOne" | "count" | "exists" | "delete"
    )
        || (policy.operation != "find"
            && (policy.page_argument.is_some() || !policy.order_by.is_empty()));
    invalid_entity || invalid_fields || invalid_sizes || invalid_limits || invalid_operation
}
fn behavior_query_predicate(
    policy: &BehaviorQueryPolicy,
    predicate: &BehaviorQueryPredicate,
    args: &[BehaviorValue],
    used: &mut std::collections::BTreeSet<usize>,
) -> BehaviorResult<BehaviorValue> {
    if !policy.fields.contains(&predicate.field) {
        return Err(behavior_invalid());
    }
    let mut values = std::collections::BTreeMap::new();
    values.insert(String::from("field"), predicate.field.into());
    values.insert(String::from("operator"), predicate.operator.into());
    match predicate.operator {
        "isNull" | "isNotNull" if predicate.argument.is_none() => {}
        "eq" | "ne" | "lt" | "lte" | "gt" | "gte" | "in" => {
            let index = predicate
                .argument
                .filter(|index| *index < 32)
                .ok_or_else(behavior_invalid)?;
            let value = args.get(index).ok_or_else(behavior_invalid)?;
            if !behavior_query_operand(predicate.operator, value) {
                return Err(behavior_invalid());
            }
            used.insert(index);
            values.insert(String::from("value"), value.clone());
        }
        _ => return Err(behavior_invalid()),
    }
    Ok(BehaviorValue::Object(values))
}
fn behavior_query_operand(operator: &str, value: &BehaviorValue) -> bool {
    if operator != "in" {
        return behavior_query_scalar(value);
    }
    let BehaviorValue::Array(values) = value else {
        return false;
    };
    values.len() <= 100 && values.iter().all(behavior_query_scalar)
}
fn behavior_query_order(policy: &BehaviorQueryPolicy) -> BehaviorResult<Vec<BehaviorValue>> {
    let mut fields = std::collections::BTreeSet::new();
    let mut order = Vec::new();
    for sort in &policy.order_by {
        if !policy.fields.contains(&sort.field)
            || !fields.insert(sort.field)
            || !matches!(sort.direction, "asc" | "desc")
        {
            return Err(behavior_invalid());
        }
        order.push(BehaviorValue::object([
            ("field", sort.field.into()),
            ("direction", sort.direction.into()),
        ]));
    }
    Ok(order)
}
fn behavior_query_page(
    policy: &BehaviorQueryPolicy,
    args: &[BehaviorValue],
    used: &mut std::collections::BTreeSet<usize>,
) -> BehaviorResult<BehaviorValue> {
    if !policy
        .order_by
        .iter()
        .any(|sort| sort.field == policy.entity.id)
    {
        return Err(behavior_invalid());
    }
    let default = BehaviorValue::Object(std::collections::BTreeMap::new());
    let raw = match policy.page_argument {
        Some(index) if index < 32 && used.insert(index) => {
            args.get(index).ok_or_else(behavior_invalid)?
        }
        None => &default,
        _ => return Err(behavior_invalid()),
    };
    let page = behavior_page(
        raw,
        &BehaviorPagePolicy {
            argument: 0,
            default_size: policy.default_size,
            max_size: policy.max_size,
            sort: policy.fields.clone(),
        },
    )?;
    if !matches!(page.get("sort"), Some(BehaviorValue::Array(sort)) if sort.is_empty()) {
        return Err(behavior_invalid());
    }
    Ok(page)
}

fn behavior_query_scalar(value: &BehaviorValue) -> bool {
    matches!(
        value,
        BehaviorValue::Null | BehaviorValue::Bool(_) | BehaviorValue::String(_)
    ) || matches!(value, BehaviorValue::Number(number) if number.is_finite())
}

fn behavior_entity_id(value: &BehaviorValue) -> BehaviorResult<()> {
    match value {
        BehaviorValue::String(value) if !value.is_empty() && value.len() <= 256 => Ok(()),
        BehaviorValue::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value.abs() <= 9_007_199_254_740_991.0 =>
        {
            Ok(())
        }
        _ => Err(behavior_invalid()),
    }
}
fn behavior_exchange_request(
    policy: &BehaviorHttpPolicy,
    args: &[BehaviorValue],
) -> BehaviorResult<BehaviorValue> {
    let argument = |index: usize| args.get(index).cloned().ok_or_else(behavior_invalid);
    let mut query = Vec::new();
    for (name, index) in policy.query {
        query.push((*name, argument(*index)?));
    }
    let mut headers = Vec::new();
    for (name, index) in policy.headers {
        headers.push((*name, argument(*index)?));
    }
    let mut descriptor = vec![
        ("query", BehaviorValue::object(query)),
        ("headers", BehaviorValue::object(headers)),
    ];
    if let Some(index) = policy.body {
        descriptor.push(("body", argument(index)?));
    }
    Ok(BehaviorValue::object(descriptor))
}

fn behavior_exchange_path(
    policy: &BehaviorHttpPolicy,
    args: &[BehaviorValue],
) -> BehaviorResult<String> {
    let mut path = policy.path.to_owned();
    for (name, index) in policy.path_variables {
        let text = match args.get(*index) {
            Some(BehaviorValue::String(value)) => value.clone(),
            Some(BehaviorValue::Bool(value)) => value.to_string(),
            Some(BehaviorValue::Number(value)) if value.fract() == 0.0 && value.abs() < 1e15 => {
                format!("{}", *value as i64)
            }
            Some(BehaviorValue::Number(value)) => value.to_string(),
            _ => return Err(behavior_invalid()),
        };
        let allowed = text.len() <= 256
            && !text.is_empty()
            && text != "."
            && text != ".."
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'~' | b'-'));
        if !allowed {
            return Err(behavior_invalid());
        }
        path = path.replace(&format!("{{{name}}}"), &text);
    }
    Ok(path)
}

fn behavior_http_request(
    policy: &BehaviorHttpPolicy,
    input: BehaviorValue,
) -> BehaviorResult<BehaviorHttpRequest> {
    if !behavior_identifier(policy.client)
        || !behavior_http_method(policy.method)
        || !behavior_safe_path(policy.path)
    {
        return Err(behavior_invalid());
    }
    let BehaviorValue::Object(mut value) = input.checked(1_048_576)? else {
        return Err(behavior_invalid());
    };
    if value
        .keys()
        .any(|key| !["query", "headers", "body"].contains(&key.as_str()))
    {
        return Err(behavior_invalid());
    }
    let query = value
        .remove("query")
        .unwrap_or_else(|| BehaviorValue::Object(Default::default()));
    let headers = value
        .remove("headers")
        .unwrap_or_else(|| BehaviorValue::Object(Default::default()));
    let body = value.remove("body");
    if ["GET", "HEAD"].contains(&policy.method) && body.is_some() {
        return Err(behavior_invalid());
    }
    if !behavior_valid_query_values(&query) || !behavior_valid_headers(&headers) {
        return Err(behavior_invalid());
    }
    Ok(BehaviorHttpRequest {
        method: policy.method,
        path: policy.path.to_owned(),
        query,
        headers,
        body,
        redirect: "error",
        credentials: "omit",
        max_response_bytes: 1_048_576,
    })
}
fn behavior_valid_query_values(query: &BehaviorValue) -> bool {
    let BehaviorValue::Object(values) = query else {
        return false;
    };
    values.values().all(|value| {
        matches!(
            value,
            BehaviorValue::String(_) | BehaviorValue::Number(_) | BehaviorValue::Bool(_)
        )
    })
}
fn behavior_valid_headers(headers: &BehaviorValue) -> bool {
    let BehaviorValue::Object(values) = headers else {
        return false;
    };
    for (key, value) in values {
        let BehaviorValue::String(value) = value else {
            return false;
        };
        if !behavior_allowed_header(key, value) {
            return false;
        }
    }
    true
}
fn behavior_allowed_header(key: &str, value: &str) -> bool {
    if key.is_empty() || key.len() > 128 || value.contains(['\r', '\n', '\0']) {
        return false;
    }
    if !key
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+.^_`|~-".contains(&byte))
    {
        return false;
    }
    let forbidden = ["host", "connection", "content-length", "transfer-encoding", "cookie"];
    !forbidden.contains(&key.to_ascii_lowercase().as_str())
}
fn behavior_safe_path(path: &str) -> bool {
    if path.len() > 2048 || !path.starts_with('/') || path.starts_with("//") {
        return false;
    }
    let mut decoded = Vec::new();
    let mut bytes = path.bytes();
    while let Some(byte) = bytes.next() {
        let byte = if byte == b'%' {
            let Some(escaped) = behavior_percent_byte(&mut bytes) else {
                return false;
            };
            escaped
        } else {
            byte
        };
        if byte.is_ascii_whitespace() || byte < 32 || b"\\?#%".contains(&byte) {
            return false;
        }
        decoded.push(byte);
    }
    let Ok(decoded) = std::str::from_utf8(&decoded) else {
        return false;
    };
    !decoded.starts_with("//")
        && !decoded.chars().any(char::is_whitespace)
        && !decoded.split('/').any(|part| matches!(part, "." | ".."))
}
fn behavior_percent_byte(bytes: &mut impl Iterator<Item = u8>) -> Option<u8> {
    let high = char::from(bytes.next()?).to_digit(16)?;
    let low = char::from(bytes.next()?).to_digit(16)?;
    Some((high * 16 + low) as u8)
}

pub struct BehaviorCalls;

impl BehaviorCalls {
    fn keys() -> &'static std::sync::Mutex<Vec<String>> {
        static KEYS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        &KEYS
    }

    pub fn record(key: &str) {
        let mut keys = Self::keys().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !keys.iter().any(|known| known == key) {
            keys.push(key.to_owned());
        }
    }

    pub fn take() -> BehaviorValue {
        let mut keys = Self::keys().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        BehaviorValue::Array(std::mem::take(&mut *keys).into_iter().map(BehaviorValue::from).collect())
    }
}
