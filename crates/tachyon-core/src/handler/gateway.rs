use super::HandlerSource;
use super::process::{Drained, HandlerCancellation, HandlerSupervisor, ProcessReaper};
use super::source::GatewaySnapshot;
use crate::Failure;
use crate::behavior::{GatewayDeclaration, GatewayLimits, GatewayTarget, gateway_target_failure};
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tokio::io::AsyncWriteExt;
use tokio::process::{ChildStdin, Command};
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};

#[derive(Clone, Debug)]
enum GatewayRoute {
    Command(Vec<String>),
    Unit(Box<HandlerSource>),
}

#[derive(Clone, Debug)]
struct GatewayEntry {
    arity: usize,
    method: String,
    route: GatewayRoute,
}

#[derive(Debug, Default)]
pub(crate) struct GatewayTable {
    entries: BTreeMap<String, GatewayEntry>,
}

impl GatewayTable {
    pub(super) fn resolve(
        snapshot: &GatewaySnapshot<'_>,
        caller: &Path,
        contents: &str,
        portable: &str,
    ) -> Result<Option<Arc<Self>>, Failure> {
        let declarations = crate::yon_cpp::CppYon::gateways(
            snapshot.source_root.path(),
            caller,
            contents,
            crate::behavior::gateway_declarations,
        )?;
        if declarations.is_empty() {
            return Ok(None);
        }
        let caller_extension = extension_of(caller);
        let mut unit_methods: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        for declaration in &declarations {
            let GatewayTarget::Class { path, method } = &declaration.target else {
                continue;
            };
            let target = ClassTarget { path, method };
            check_class_target(
                snapshot.source_root.path(),
                portable,
                caller_extension,
                &target,
                declaration.arity,
            )?;
            let methods = unit_methods.entry(path.clone()).or_default();
            if let Some(previous) = methods.insert(method.clone(), declaration.arity)
                && previous != declaration.arity
            {
                return Err(gateway_target_failure(
                    portable,
                    0,
                    &format!(
                        "Gateway target {path}.{method} is requested with different argument counts"
                    ),
                ));
            }
        }
        let mut units = BTreeMap::new();
        for (path, methods) in &unit_methods {
            units.insert(path.clone(), discover_unit(snapshot, path, methods)?);
        }
        let mut table = Self::default();
        for declaration in &declarations {
            table.add(snapshot.source_root.path(), portable, declaration, &units)?;
        }
        Ok(Some(Arc::new(table)))
    }

    fn add(
        &mut self,
        snapshot: &Path,
        portable: &str,
        declaration: &GatewayDeclaration,
        units: &BTreeMap<String, HandlerSource>,
    ) -> Result<(), Failure> {
        let entry = match &declaration.target {
            GatewayTarget::Command { arguments } => {
                check_command_files(snapshot, portable, arguments)?;
                GatewayEntry {
                    arity: declaration.arity,
                    method: declaration.method.clone(),
                    route: GatewayRoute::Command(arguments.clone()),
                }
            }
            GatewayTarget::Class { path, method } => GatewayEntry {
                arity: declaration.arity,
                method: method.clone(),
                route: GatewayRoute::Unit(Box::new(units[path].clone())),
            },
        };
        if self
            .entries
            .insert(declaration.key.clone(), entry)
            .is_some()
        {
            return Err(gateway_target_failure(
                portable,
                0,
                &format!("Gateway '{}' is declared twice", declaration.key),
            ));
        }
        Ok(())
    }

    pub(crate) fn programs(&self) -> Vec<&str> {
        let mut programs = Vec::new();
        for entry in self.entries.values() {
            if let GatewayRoute::Command(arguments) = &entry.route {
                programs.push(arguments[0].as_str());
            }
        }
        programs
    }

    pub(crate) fn units(&self) -> Vec<&HandlerSource> {
        let mut units = Vec::new();
        for entry in self.entries.values() {
            if let GatewayRoute::Unit(unit) = &entry.route {
                units.push(unit.as_ref());
            }
        }
        units
    }
}

struct ClassTarget<'a> {
    path: &'a str,
    method: &'a str,
}

fn extension_of(path: &Path) -> &str {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
}

fn is_role_file_path(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() < 3 || segments[0] != "server" {
        return false;
    }
    let role_folder = crate::annotation_modules::role_folders()
        .iter()
        .any(|folder| *folder == segments[1] && *folder != "controllers");
    role_folder
        && segments
            .iter()
            .all(|segment| !matches!(*segment, "" | "." | ".."))
}

fn check_class_target(
    snapshot: &Path,
    portable: &str,
    caller_extension: &str,
    target: &ClassTarget<'_>,
    arity: usize,
) -> Result<(), Failure> {
    let path = target.path;
    let failed = |message: String| Err(gateway_target_failure(portable, 0, &message));
    if !is_role_file_path(path) {
        return failed(format!(
            "Gateway class '{path}' is not a role file under a server role folder"
        ));
    }
    let target_path = Path::new(path);
    let extension = extension_of(target_path);
    if !crate::convention::is_yon_language(target_path) {
        return failed(format!(
            "Gateway class '{path}' is not written in a Yon language"
        ));
    }
    if extension == caller_extension {
        return failed(format!(
            "Gateway class '{path}' is in the caller's own language; inject it instead"
        ));
    }
    let Ok(text) = std::fs::read_to_string(snapshot.join(path)) else {
        return failed(format!(
            "Gateway class '{path}' is not in the captured server snapshot"
        ));
    };
    check_class_method(portable, target, &text, arity)
}

fn check_class_method(
    portable: &str,
    target: &ClassTarget<'_>,
    text: &str,
    arity: usize,
) -> Result<(), Failure> {
    let path = Path::new(target.path);
    let class = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    let method = target.method;
    let failed = |message: String| Err(gateway_target_failure(portable, 0, &message));
    let signature = crate::behavior::gateway_target_signature(path, text, class, method, arity)?;
    let Some(signature) = signature.filter(|found| found.has_body && !found.is_static) else {
        return failed(missing_method_message(path, class, method));
    };
    if signature.parameter_types.len() != arity {
        return failed(format!(
            "Gateway target {class}.{method} takes {} arguments but the caller declares {arity}",
            signature.parameter_types.len()
        ));
    }
    if !signature.uses_json_values(extension_of(path)) {
        return failed(format!(
            "Gateway target {class}.{method} must take and return the language's JSON value type"
        ));
    }
    if !signature.has_supported_receiver {
        return failed(format!(
            "Gateway target {class}.{method} must use &self; managed Rust targets cannot consume self or borrow &mut self"
        ));
    }
    let declarations = crate::behavior::gateway_declarations(path, text)?;
    if declarations
        .iter()
        .any(|declaration| declaration.owner == class)
    {
        return failed(format!(
            "Gateway target {class} declares a gateway itself; only one hop is allowed"
        ));
    }
    Ok(())
}

fn missing_method_message(path: &Path, class: &str, method: &str) -> String {
    if extension_of(path) != "go" {
        return format!("Gateway target {class}.{method} is not a concrete instance method");
    }
    let exported_name = crate::go_source::upper_first(method);
    format!(
        "Gateway target {class}.{method} needs the exported Go method {exported_name} on {class}; other languages call a Go method by its name with an upper-case first letter"
    )
}

fn discover_unit(
    snapshot: &GatewaySnapshot<'_>,
    path: &str,
    methods: &BTreeMap<String, usize>,
) -> Result<HandlerSource, Failure> {
    let target_path = Path::new(path);
    let class = target_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    let mut signatures = Vec::new();
    for (method, arity) in methods {
        signatures.push(format!("{method}:{arity}"));
    }
    let identity = format!("{path}\n{}", signatures.join(","));
    let digest = crate::compiler::hex_digest(Sha256::digest(identity.as_bytes()));
    let unit = PathBuf::from(format!(
        "server/gateways/{}/yon.{}",
        &digest[..16],
        extension_of(target_path)
    ));
    HandlerSource::discover_gateway_unit(snapshot, &unit, class, methods)
}

fn check_command_files(
    snapshot: &Path,
    portable: &str,
    arguments: &[String],
) -> Result<(), Failure> {
    for entry in &arguments[1..] {
        if !GatewayTarget::names_a_path(entry) {
            continue;
        }
        if !is_helper_file(snapshot, entry) {
            return Err(gateway_target_failure(
                portable,
                0,
                &format!("Gateway command file '{entry}' is not a file under server/helpers"),
            ));
        }
    }
    Ok(())
}

fn is_helper_file(snapshot: &Path, entry: &str) -> bool {
    let Some(rest) = entry.strip_prefix("server/helpers/") else {
        return false;
    };
    let plain = entry
        .split('/')
        .all(|segment| !matches!(segment, "" | "." | ".."));
    if rest.is_empty() || !plain {
        return false;
    }
    std::fs::symlink_metadata(snapshot.join(entry)).is_ok_and(|metadata| metadata.is_file())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GatewayFailure {
    Invalid,
    Timeout,
    Capacity,
    Adapter,
}

impl GatewayFailure {
    const fn name(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::Timeout => "timeout",
            Self::Capacity => "capacity",
            Self::Adapter => "adapter",
        }
    }
}

pub(super) struct GatewayCalls {
    supervisor: HandlerSupervisor,
    table: Arc<GatewayTable>,
    request_id: String,
    working_directory: PathBuf,
    deadline: Instant,
    cancellation: HandlerCancellation,
    calls: AtomicUsize,
}

pub(super) struct GatewayCallSetup {
    pub supervisor: HandlerSupervisor,
    pub table: Arc<GatewayTable>,
    pub request_id: String,
    pub working_directory: PathBuf,
    pub deadline: Instant,
    pub cancellation: HandlerCancellation,
}

impl GatewayCalls {
    pub(super) fn new(setup: GatewayCallSetup) -> Self {
        Self {
            supervisor: setup.supervisor,
            table: setup.table,
            request_id: setup.request_id,
            working_directory: setup.working_directory,
            deadline: setup.deadline,
            cancellation: setup.cancellation,
            calls: AtomicUsize::new(0),
        }
    }

    pub(super) async fn answer(&self, command: &Value) -> Vec<u8> {
        let answer = match self.call(command).await {
            Ok(result) => json!({ "result": result }),
            Err(failure) => self.failed(failure),
        };
        let bytes = serde_json::to_vec(&answer).unwrap_or_default();
        if bytes.len() <= GatewayLimits::MAX_BYTES {
            return bytes;
        }
        let failed = self.failed(GatewayFailure::Adapter);
        serde_json::to_vec(&failed).unwrap_or_default()
    }

    fn failed(&self, failure: GatewayFailure) -> Value {
        eprintln!("{}", self.failure_event(failure));
        json!({ "error": failure.name() })
    }

    fn failure_event(&self, failure: GatewayFailure) -> Value {
        json!({
            "event": "handler.gateway_failed",
            "request_id": self.request_id,
            "category": failure.name(),
        })
    }

    async fn call(&self, command: &Value) -> Result<Value, GatewayFailure> {
        let previous_calls = self.calls.fetch_add(1, Ordering::SeqCst);
        if previous_calls >= GatewayLimits::MAX_CALLS {
            return Err(GatewayFailure::Capacity);
        }
        let (entry, arguments) = self.requested(command)?;
        if Instant::now() >= self.deadline || self.cancellation.is_cancelled() {
            return Err(GatewayFailure::Timeout);
        }
        let input = json!({ "method": entry.method, "arguments": arguments });
        match &entry.route {
            GatewayRoute::Command(program) => self.run_command(program, &input).await,
            GatewayRoute::Unit(unit) => self.call_unit(unit, &input, previous_calls).await,
        }
    }

    async fn call_unit(
        &self,
        unit: &HandlerSource,
        input: &Value,
        call_number: usize,
    ) -> Result<Value, GatewayFailure> {
        let time_left = self.deadline.saturating_duration_since(Instant::now());
        let milliseconds = u64::try_from(time_left.as_millis()).unwrap_or(u64::MAX);
        if milliseconds == 0 {
            return Err(GatewayFailure::Timeout);
        }
        let prefix: String = self.request_id.chars().take(96).collect();
        let request_id = format!("{prefix}-gateway-{call_number}");
        let mut request = HandlerRequest::route(request_id, "/", HttpMethod::Post);
        request.operation = String::from("gateway.invoke");
        request.deadline_ms = Some(milliseconds);
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: input.to_string(),
        });
        let response = self
            .supervisor
            .invoke_gateway_unit(unit, &request, &self.cancellation)
            .await;
        match response {
            Ok(response) => unit_result(&response),
            Err(failure) if is_deadline_or_cancel(&failure) => Err(GatewayFailure::Timeout),
            Err(_) => Err(GatewayFailure::Adapter),
        }
    }

    fn requested<'a>(
        &'a self,
        command: &'a Value,
    ) -> Result<(&'a GatewayEntry, &'a Value), GatewayFailure> {
        let Some(object) = command.as_object() else {
            return Err(GatewayFailure::Invalid);
        };
        if object.len() != 3 {
            return Err(GatewayFailure::Invalid);
        }
        let key = object.get("key").and_then(Value::as_str);
        let Some(entry) = key.and_then(|key| self.table.entries.get(key)) else {
            return Err(GatewayFailure::Invalid);
        };
        let Some(arguments) = object.get("arguments") else {
            return Err(GatewayFailure::Invalid);
        };
        let count = arguments.as_array().map(Vec::len);
        if count != Some(entry.arity) || !within_bounds(arguments) {
            return Err(GatewayFailure::Invalid);
        }
        Ok((entry, arguments))
    }

    async fn run_command(
        &self,
        program: &[String],
        input: &Value,
    ) -> Result<Value, GatewayFailure> {
        let permit = self
            .supervisor
            .acquire_gateway_permit(&self.cancellation, self.deadline)
            .await
            .map_err(|_| GatewayFailure::Timeout)?;
        let result = self.run_command_process(program, input).await;
        drop(permit);
        result
    }

    async fn run_command_process(
        &self,
        program: &[String],
        input: &Value,
    ) -> Result<Value, GatewayFailure> {
        if Instant::now() >= self.deadline || self.cancellation.is_cancelled() {
            return Err(GatewayFailure::Timeout);
        }
        let Some(mut running) = self.start_command(program, input).await else {
            return Err(GatewayFailure::Adapter);
        };
        let status = tokio::select! {
            status = running.child.inner().wait() => status.ok(),
            () = self.cancellation.cancelled() => None,
            () = sleep_until(self.deadline) => None,
        };
        ProcessReaper::terminate(&mut running.child, &self.request_id).await;
        running.input.abort();
        let _input_stopped = running.input.await;
        let settled = Drained::settle_pair(running.stdout, running.stderr, self.deadline).await;
        let Some(status) = status else {
            return Err(GatewayFailure::Timeout);
        };
        let Ok((stdout, stderr)) = settled else {
            return Err(GatewayFailure::Timeout);
        };
        if !status.success() || stderr.overflow || stderr.read_error {
            return Err(GatewayFailure::Adapter);
        }
        command_result(&stdout)
    }

    async fn start_command(&self, program: &[String], input: &Value) -> Option<RunningCommand> {
        let mut command = Command::new(&program[0]);
        command
            .args(&program[1..])
            .current_dir(&self.working_directory);
        self.supervisor
            .external_environment()
            .apply_external(command.as_std_mut());
        HandlerSupervisor::pipe_all_streams(&mut command);
        let mut child = command.group().kill_on_drop(true).spawn().ok()?;
        let process = child.inner();
        let (Some(stdin), Some(stdout), Some(stderr)) = (
            process.stdin.take(),
            process.stdout.take(),
            process.stderr.take(),
        ) else {
            ProcessReaper::terminate(&mut child, &self.request_id).await;
            return None;
        };
        Some(RunningCommand {
            child,
            stdout: tokio::spawn(Drained::read_all(stdout, GatewayLimits::MAX_BYTES + 1)),
            stderr: tokio::spawn(Drained::read_all(stderr, self.supervisor.stderr_limit())),
            input: write_input(stdin, serde_json::to_vec(input).unwrap_or_default()),
        })
    }
}

struct RunningCommand {
    child: AsyncGroupChild,
    stdout: JoinHandle<Drained>,
    stderr: JoinHandle<Drained>,
    input: JoinHandle<()>,
}

fn write_input(mut stdin: ChildStdin, bytes: Vec<u8>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let _written = stdin.write_all(&bytes).await;
        let _closed = stdin.shutdown().await;
    })
}

fn is_deadline_or_cancel(failure: &Failure) -> bool {
    failure
        .diagnostics()
        .iter()
        .any(|diagnostic| matches!(diagnostic.code.number(), 2110 | 2111))
}

fn unit_result(response: &HandlerResponse) -> Result<Value, GatewayFailure> {
    if response.status != 200 || response.error.is_some() {
        return Err(GatewayFailure::Adapter);
    }
    let Some(body) = &response.body else {
        return Err(GatewayFailure::Adapter);
    };
    if body.encoding != HandlerBodyEncoding::Utf8 {
        return Err(GatewayFailure::Adapter);
    }
    let Ok(Value::Object(mut answer)) = serde_json::from_str::<Value>(&body.data) else {
        return Err(GatewayFailure::Adapter);
    };
    let Some(result) = answer.remove("result").filter(|_| answer.is_empty()) else {
        return Err(GatewayFailure::Adapter);
    };
    if !within_bounds(&result) {
        return Err(GatewayFailure::Adapter);
    }
    Ok(result)
}

fn command_result(stdout: &Drained) -> Result<Value, GatewayFailure> {
    if stdout.overflow || stdout.read_error {
        return Err(GatewayFailure::Adapter);
    }
    let Ok(text) = std::str::from_utf8(&stdout.bytes) else {
        return Err(GatewayFailure::Adapter);
    };
    let Ok(value) = serde_json::from_str::<Value>(text.trim()) else {
        return Err(GatewayFailure::Adapter);
    };
    if !within_bounds(&value) {
        return Err(GatewayFailure::Adapter);
    }
    Ok(value)
}

fn within_bounds(value: &Value) -> bool {
    let mut nodes = 0;
    counted_within_bounds(value, 0, &mut nodes)
}

fn counted_within_bounds(value: &Value, depth: usize, nodes: &mut usize) -> bool {
    *nodes += 1;
    if *nodes > GatewayLimits::MAX_NODES || depth > GatewayLimits::MAX_DEPTH {
        return false;
    }
    match value {
        Value::Array(items) => items
            .iter()
            .all(|item| counted_within_bounds(item, depth + 1, nodes)),
        Value::Object(fields) => fields
            .values()
            .all(|item| counted_within_bounds(item, depth + 1, nodes)),
        _ => true,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{
        ClassTarget, GatewayCallSetup, GatewayCalls, GatewayFailure, GatewayTable,
        check_class_method, check_command_files, is_helper_file, within_bounds,
    };
    use crate::handler::{HandlerCancellation, HandlerSupervisor, HandlerSupervisorOptions};
    use serde_json::{Value, json};
    use std::sync::Arc;
    use tokio::time::{Duration, Instant};

    fn empty_calls() -> GatewayCalls {
        GatewayCalls::new(GatewayCallSetup {
            supervisor: HandlerSupervisor::new(HandlerSupervisorOptions::default())
                .expect("supervisor"),
            table: Arc::new(GatewayTable::default()),
            request_id: String::from("request-7"),
            working_directory: std::env::temp_dir(),
            deadline: Instant::now() + Duration::from_secs(5),
            cancellation: HandlerCancellation::default(),
        })
    }

    #[tokio::test]
    async fn a_gateway_failure_logs_only_the_event_request_and_category() {
        let calls = empty_calls();
        for failure in [
            GatewayFailure::Invalid,
            GatewayFailure::Timeout,
            GatewayFailure::Capacity,
            GatewayFailure::Adapter,
        ] {
            let event = calls.failure_event(failure);
            let expected = json!({
                "event": "handler.gateway_failed",
                "request_id": "request-7",
                "category": failure.name(),
            });
            assert_eq!(event, expected);
        }
        let unknown = json!({"operation": "gateway", "key": "Missing.call", "arguments": []});
        let answer: Value = serde_json::from_slice(&calls.answer(&unknown).await).expect("JSON");
        assert_eq!(answer, json!({"error": "invalid"}));
    }

    fn nested(depth: usize) -> Value {
        let mut value = json!(1);
        for _level in 0..depth {
            value = json!([value]);
        }
        value
    }

    #[test]
    fn json_arguments_are_bounded_by_depth_and_node_count() {
        assert!(within_bounds(&nested(32)));
        assert!(!within_bounds(&nested(33)));
        assert!(within_bounds(&Value::Array(vec![json!(0); 9_999])));
        assert!(!within_bounds(&Value::Array(vec![json!(0); 10_000])));
    }

    #[test]
    fn command_files_stay_inside_server_helpers() {
        let snapshot = tempfile::tempdir().expect("snapshot");
        let helpers = snapshot.path().join("server/helpers");
        std::fs::create_dir_all(helpers.join("nested")).expect("helpers");
        std::fs::write(helpers.join("report.rb"), "puts 1").expect("helper");
        std::fs::write(snapshot.path().join("server/other.rb"), "puts 1").expect("other");
        assert!(is_helper_file(snapshot.path(), "server/helpers/report.rb"));
        for drive_relative in ["C:report.rb", "c:server/helpers/report.rb"] {
            let arguments = [String::from("ruby"), String::from(drive_relative)];
            let refused = check_command_files(snapshot.path(), "server/remotes/A.js", &arguments);
            assert!(refused.is_err(), "{drive_relative}");
        }
        for refused in [
            "server/helpers/missing.rb",
            "server/helpers/nested",
            "server/helpers/../other.rb",
            "server/other.rb",
            "/server/helpers/report.rb",
            "server/helpers/./report.rb",
            "server/helpers/",
        ] {
            assert!(!is_helper_file(snapshot.path(), refused), "{refused}");
        }
    }

    fn go_target_failure(methods: &str) -> Option<String> {
        let text = format!("// @Service\ntype ReportService struct{{}}\n\n{methods}");
        let target = ClassTarget {
            path: "server/services/ReportService.go",
            method: "summarize",
        };
        let checked = check_class_method("server/remotes/ReportClient.py", &target, &text, 2);
        checked.err().map(|failure| failure.to_string())
    }

    #[test]
    fn a_go_target_is_an_exported_method_with_a_value_or_pointer_receiver() {
        for receiver in [
            "service *ReportService",
            "service ReportService",
            "*ReportService",
        ] {
            let method = format!(
                "func ({receiver}) Summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {{\n\treturn detail, nil\n}}\n"
            );
            assert_eq!(go_target_failure(&method), None, "{receiver}");
        }
        let plain = "func (service *ReportService) Summarize(mode, detail BehaviorValue) BehaviorValue {\n\treturn detail\n}\n";
        assert_eq!(go_target_failure(plain), None);
    }

    #[test]
    fn go_targets_fail_with_a_ty1312_that_names_the_fix() {
        let summarize = "func (service *ReportService) Summarize";
        let cases = [
            (
                String::from(
                    "func (service *ReportService) summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {\n\treturn detail, nil\n}\n",
                ),
                "exported Go method Summarize",
            ),
            (
                String::from(
                    "func (service *OtherService) Summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {\n\treturn detail, nil\n}\n",
                ),
                "exported Go method Summarize",
            ),
            (
                format!(
                    "{summarize}(mode BehaviorValue) (BehaviorValue, error) {{\n\treturn mode, nil\n}}\n"
                ),
                "takes 1 arguments but the caller declares 2",
            ),
            (
                format!(
                    "{summarize}(mode string, detail BehaviorValue) (BehaviorValue, error) {{\n\treturn detail, nil\n}}\n"
                ),
                "JSON value type",
            ),
            (
                format!(
                    "{summarize}(mode, detail BehaviorValue) (value BehaviorValue, failure error) {{\n\treturn detail, nil\n}}\n"
                ),
                "JSON value type",
            ),
            (
                format!(
                    "{summarize}(mode, detail BehaviorValue) BehaviorValue {{\n\treturn mode\n}}\n\n{summarize}(mode, detail BehaviorValue) BehaviorValue {{\n\treturn detail\n}}\n"
                ),
                "ambiguous",
            ),
        ];
        for (methods, expected) in cases {
            let failure = go_target_failure(&methods).expect("refused Go target");
            assert!(failure.contains("TY1312"), "{methods}: {failure}");
            assert!(failure.contains(expected), "{methods}: {failure}");
        }
    }
}
