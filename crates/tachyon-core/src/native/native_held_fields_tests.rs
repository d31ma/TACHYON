#![allow(clippy::expect_used)]

use super::native_page_graph_tests::NativePageGraphCheck;

struct HeldFieldsCheck;

impl HeldFieldsCheck {
    const PROVIDER: &'static str = r#"
#[Service]
struct CounterService {
    #[LocalState]
    visits: std::sync::Mutex<i64>,
    #[SessionState]
    draft: std::sync::RwLock<String>,
    picker: FilePicker,
}
impl CounterService {
    fn new() -> Self {
        Self { visits: std::sync::Mutex::new(0), draft: std::sync::RwLock::new(String::new()), picker: FilePicker::default() }
    }
    pub fn visit(&self) { *self.visits.lock().unwrap_or_else(|error| error.into_inner()) += 1; }
    pub fn read(&self) -> i64 { *self.visits.lock().unwrap_or_else(|error| error.into_inner()) }
    pub fn draft(&self) -> String { self.draft.read().unwrap_or_else(|error| error.into_inner()).clone() }
    #[OnStart]
    pub fn started(&self) { let _ = self.picker.scope.call("probe.counter.start", Vec::new()); }
    #[OnStop]
    pub fn stopped(&self) { let _ = self.picker.scope.call("probe.counter.stop", Vec::new()); }
}
"#;

    const WRITER: &'static str = r"
#[Service]
#[NeedsAll]
struct WriterService { counter: std::sync::Arc<CounterService> }
impl WriterService {
    pub fn visit(&self) { self.counter.visit(); }
    pub fn read(&self) -> i64 { self.counter.read() }
    pub fn draft(&self) -> String { self.counter.draft() }
}
";

    const READER: &'static str = r"
#[Service]
#[derive(Default)]
struct ReaderService { count: i64 }
impl ReaderService {
    pub fn read(&self) -> i64 { self.count }
}
";

    const CALLER: &'static str = r###"
struct Probe;
impl Probe {
    fn tape() -> &'static std::thread::LocalKey<core::cell::RefCell<Vec<String>>> {
        thread_local! { static VALUE: core::cell::RefCell<Vec<String>> = const { core::cell::RefCell::new(Vec::new()) }; }
        &VALUE
    }
    fn count(event: &str) -> usize { Self::tape().with(|tape| tape.borrow().iter().filter(|value| value.as_str() == event).count()) }
}
fn request(service: &str, owner: &str, rest: &str) -> String {
    tac_handle(&format!(r##"{{"route":"/","service":"{service}","owner":"{owner}",{rest}}}"##))
}
fn main() {
    NativeCapabilities::register(|operation, _| {
        if operation.starts_with("probe.") { Probe::tape().with(|tape| tape.borrow_mut().push(operation.to_owned())); }
        String::from("{\"status\":\"cancelled\"}")
    });
    let writer = "00000000-0000-4000-8000-000000000001";
    let reader = "00000000-0000-4000-8000-000000000002";
    let holder = "00000000-0000-4000-8000-000000000003";
    let refused = "00000000-0000-4000-8000-000000000004";
    let answer = request("writer", writer, r#""op":"init""#);
    assert!(answer.contains(r#""needs":["counter"]"#), "the root lists the service it holds: {answer}");
    let answer = request("reader", reader, r#""op":"init""#);
    assert!(answer.contains(r#""needs":[]"#), "a root that holds nothing lists nothing: {answer}");
    let _ = request("writer", writer, r#""op":"call","name":"visit","args":[]"#);
    let answer = request("writer", holder, r#""op":"init","props":{},"held":"counter""#);
    assert!(answer.contains(r#""fields":["visits","draft"]"#), "the holder shows only the saved fields: {answer}");
    assert!(answer.contains(r#""methods":[]"#) && answer.contains(r#""listeners":[]"#), "the holder has no methods and no events: {answer}");
    assert_eq!(request("writer", holder, r#""op":"get","name":"visits""#), r#"{"value":1}"#);
    assert_eq!(request("writer", holder, r#""op":"set","name":"visits","value":5"#), r#"{"value":null}"#);
    assert_eq!(request("writer", writer, r#""op":"call","name":"read","args":[]"#), r#"{"value":5}"#, "the holder reaches the same Arc as the root");
    assert_eq!(request("writer", holder, r#""op":"set","name":"draft","value":"kept draft""#), r#"{"value":null}"#);
    assert_eq!(request("writer", writer, r#""op":"call","name":"draft","args":[]"#), r#"{"value":"kept draft"}"#);
    let _ = request("writer", writer, r#""op":"call","name":"visit","args":[]"#);
    assert_eq!(request("writer", holder, r#""op":"get","name":"visits""#), r#"{"value":6}"#, "the holder reads the value the root changed");
    assert_eq!(Probe::count("probe.counter.start"), 1, "the holder does not start the service again");
    let answer = request("reader", refused, r#""op":"init","props":{},"held":"counter""#);
    assert!(answer.contains("error"), "a root that does not hold the service cannot open it: {answer}");
    let answer = request("writer", refused, r#""op":"init","props":{},"held":"writer""#);
    assert!(answer.contains("error"), "only a held service with saved fields opens: {answer}");
    let _ = request("writer", holder, r#""op":"dispose""#);
    assert_eq!(Probe::count("probe.counter.stop"), 0, "closing the holder does not stop the service");
    assert_eq!(request("writer", writer, r#""op":"call","name":"read","args":[]"#), r#"{"value":6}"#);
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    assert_eq!(Probe::count("probe.counter.stop"), 1, "the page exit stops the service once");
}
"###;
}

#[tokio::test]
async fn native_rust_held_service_keeps_its_saved_fields_through_a_field_only_holder() {
    let root = tempfile::tempdir().expect("fixture");
    let source = NativePageGraphCheck::stage_roots(
        root.path(),
        HeldFieldsCheck::PROVIDER,
        [HeldFieldsCheck::WRITER, HeldFieldsCheck::READER],
    )
    .expect("staging");
    NativePageGraphCheck::execute(&source, HeldFieldsCheck::CALLER).await;
}

#[test]
fn native_rust_held_service_refuses_a_saved_field_the_page_cannot_restore() {
    let root = tempfile::tempdir().expect("fixture");
    let provider = HeldFieldsCheck::PROVIDER
        .replace(
            "visits: std::sync::Mutex<i64>,",
            "visits: i64,\n    lock: std::sync::Mutex<i64>,",
        )
        .replace(
            "Self { visits: std::sync::Mutex::new(0),",
            "Self { visits: 0, lock: std::sync::Mutex::new(0),",
        )
        .replace("*self.visits.lock()", "*self.lock.lock()");
    let failure = NativePageGraphCheck::stage_roots(
        root.path(),
        &provider,
        [HeldFieldsCheck::WRITER, HeldFieldsCheck::READER],
    )
    .expect_err("a plain saved field on an Arc service cannot be restored");
    let message = failure.to_string();
    assert!(message.contains("Saved field 'visits: i64'"), "{message}");
    assert!(message.contains("std::sync::Mutex<T>"), "{message}");
}
