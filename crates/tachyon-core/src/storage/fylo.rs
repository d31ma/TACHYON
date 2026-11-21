mod busy_operation;
mod collection_gates;
mod collection_name;
mod collection_pass;
mod document_commands;
mod fylo_input;
mod fylo_location;
mod fylo_pipes;
mod fylo_process;
mod fylo_transactions;
mod interruption;
mod live_input;
mod namespace_name;
mod open_fylo_transaction;
mod operation_timeout;
mod patch_queue;
mod queue_commands;
mod queue_name;
mod repository_commands;
mod request_watch;
mod root_lock_wait;
mod scroll_request;
mod sql_statement;
mod startup_requests;
mod stderr_tail;
mod store_protocol;
mod store_query;
mod store_rows;
mod store_target;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};
use tokio::sync::Mutex;

use super::dialect::is_simple_identifier;
use super::{StoreCall, failure, object_keys, scalar, text};
use crate::Failure;
use crate::failure::diagnostic;
use busy_operation::BusyOperation;
pub(super) use collection_name::CollectionName;
use collection_pass::CollectionPass;
use document_commands::DocumentCommands;
use futures_util::future::OptionFuture;
use fylo_location::FyloLocation;
use fylo_process::FyloProcess;
use fylo_transactions::FyloTransactions;
use live_input::LiveInput;
use operation_timeout::OperationTimeout;
use patch_queue::{PatchAnswer, PatchQueue};
use queue_commands::QueueCommands;
use repository_commands::RepositoryCommands;
pub(super) use sql_statement::SqlStatement;
use startup_requests::StartupRequests;
use store_protocol::StoreProtocol;
use store_rows::StoreRows;
use store_target::StoreTarget;

pub(super) struct FyloStore {
    root: PathBuf,
    namespace: Option<String>,
    operation_timeout: Duration,
    process: Mutex<Option<FyloProcess>>,
    busy: BusyOperation,
    first_requests: StartupRequests,
    live_input: LiveInput,
    patches: PatchQueue,
    closed: AtomicBool,
    transactions: FyloTransactions,
}

impl FyloStore {
    const MIN_SEND_TIME: Duration = Duration::from_millis(100);
    const COMMIT: &'static str = "commit";
    const MAX_SQL_PARAMETERS: usize = 64;

    pub(super) async fn connect(settings: &Value) -> Result<Self, Failure> {
        object_keys(settings, &["driver", "url", "timeout"])?;
        let operation_timeout = OperationTimeout::from_settings(settings)?;
        let location = FyloLocation::parsed(text(settings, "url")?)?;
        let path = location.path();
        let namespace = location.namespace().map(str::to_owned);
        if path.contains('\0') || path.contains("..") {
            return Err(failure("FYLO root path contains an unsafe segment."));
        }
        let root = std::fs::canonicalize(Path::new(path))
            .map(Self::without_verbatim_prefix)
            .map_err(|_| failure("FYLO root directory must already exist."))?;
        if !root.is_dir() {
            return Err(failure("FYLO root must be a directory."));
        }
        let mut process =
            FyloProcess::spawn(&root, namespace.as_deref()).map_err(FyloError::into_failure)?;
        let identity = process
            .handshake(&json!({ "op": "handshake" }))
            .await
            .map_err(FyloError::into_failure)?;
        StoreProtocol::check(&identity)?;
        Ok(Self {
            root,
            namespace,
            operation_timeout,
            process: Mutex::new(Some(process)),
            busy: BusyOperation::new(),
            first_requests: StartupRequests::new(),
            live_input: LiveInput::new(),
            patches: PatchQueue::new(),
            closed: AtomicBool::new(false),
            transactions: FyloTransactions::new(),
        })
    }

    fn without_verbatim_prefix(root: PathBuf) -> PathBuf {
        let drive_path = root
            .to_str()
            .and_then(|text| text.strip_prefix(r"\\?\"))
            .filter(|rest| rest.get(1..2) == Some(":"))
            .map(PathBuf::from);
        drive_path.unwrap_or(root)
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        let target = StoreTarget::of(command);
        let _pass = self.pass_outside_transactions(&target).await;
        self.run(command, None).await
    }

    pub(super) async fn execute_in(&self, id: &str, command: &Value) -> Result<Value, Failure> {
        let target = StoreTarget::of(command);
        target.refuse_unless_it_joins()?;
        let joined = self.joined_handle(id, &target).await?;
        self.run(command, joined.as_deref()).await
    }

    pub(super) fn begin(&self) -> Result<Value, Failure> {
        let id = self.transactions.begin()?;
        Ok(json!({ "transaction": id }))
    }

    pub(super) async fn settle(&self, id: &str, action: &str) -> Result<Value, Failure> {
        let handle = self.transactions.settle(id)?;
        self.settle_in_fylo(handle, action).await?;
        Ok(json!({ "transaction": id, "action": action }))
    }

    async fn pass_outside_transactions(&self, target: &StoreTarget) -> CollectionPass {
        let gates = self.transactions.gates();
        match (target.sql_writes(), target.writes(), target.collection()) {
            (true, _, _) => gates.sql_writer().await,
            (false, true, Some(collection)) => gates.writer(collection).await,
            _ => CollectionPass::none(),
        }
    }

    async fn run(&self, command: &Value, joined: Option<&str>) -> Result<Value, Failure> {
        let operation = text(command, "operation")?;
        match operation {
            "sql" => self.sql(command).await,
            "document" => DocumentCommands::new(self, joined).execute(command).await,
            "queue" => QueueCommands::new(self).execute(command).await,
            _ => {
                RepositoryCommands::new(self, joined)
                    .execute(command, operation)
                    .await
            }
        }
    }

    async fn joined_handle(
        &self,
        id: &str,
        target: &StoreTarget,
    ) -> Result<Option<String>, Failure> {
        match self.transactions.binding(id)? {
            Some((collection, handle)) => self.handle_for(id, target, &collection, handle).await,
            None => self.bind_when_writing(id, target).await,
        }
    }

    async fn handle_for(
        &self,
        id: &str,
        target: &StoreTarget,
        collection: &str,
        handle: String,
    ) -> Result<Option<String>, Failure> {
        let same = target.collection() == Some(collection);
        match (same, target.writes()) {
            (true, _) => Ok(Some(handle)),
            (false, false) => Ok(None),
            (false, true) => Err(self.end_for_a_second_collection(id, handle).await),
        }
    }

    async fn end_for_a_second_collection(&self, id: &str, handle: String) -> Failure {
        self.transactions.forget(id);
        let _rolled_back_or_already_gone = self.settle_in_fylo(Some(handle), "rollback").await;
        failure(
            "A FYLO transaction covers one collection; this write names a second collection, so Tachyon rolled the transaction back.",
        )
    }

    async fn bind_when_writing(
        &self,
        id: &str,
        target: &StoreTarget,
    ) -> Result<Option<String>, Failure> {
        let Some(collection) = target.collection().filter(|_| target.writes()) else {
            return Ok(None);
        };
        self.bind(id, CollectionName::checked(collection)?)
            .await
            .map(Some)
    }

    async fn bind(&self, id: &str, collection: &str) -> Result<String, Failure> {
        let expires = self.transactions.expires(id)?;
        let pass = self.transactions.gates().transaction(collection).await;
        let budget = StoreCall::time_left(false).unwrap_or(Duration::MAX);
        let left = expires
            .saturating_duration_since(Instant::now())
            .min(budget);
        let milliseconds = u64::try_from(left.as_millis()).unwrap_or(u64::MAX);
        let request = json!({ "collection": collection, "timeoutMs": milliseconds });
        let answer = self
            .call("transactionBegin", request)
            .await
            .map_err(FyloError::into_failure)?;
        let handle = Self::transaction_handle(&answer)?;
        self.transactions
            .bind(id, collection, handle.clone(), pass)?;
        Ok(handle)
    }

    fn transaction_handle(answer: &Value) -> Result<String, Failure> {
        answer["transaction"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| failure("FYLO did not answer a transaction handle."))
    }

    async fn settle_in_fylo(&self, handle: Option<String>, action: &str) -> Result<(), Failure> {
        let Some(handle) = handle else {
            return Ok(());
        };
        let operation = Self::settle_operation(action);
        let answer = self.call(operation, json!({ "transaction": handle })).await;
        Self::settled(answer, action)
    }

    fn settle_operation(action: &str) -> &'static str {
        if action == Self::COMMIT {
            return "transactionCommit";
        }
        "transactionRollback"
    }

    fn settled(answer: Result<Value, FyloError>, action: &str) -> Result<(), Failure> {
        let gone_already = answer
            .as_ref()
            .err()
            .is_some_and(FyloError::transaction_gone);
        match (answer, action == Self::COMMIT, gone_already) {
            (Ok(_), _, _) | (Err(_), false, true) => Ok(()),
            (Err(error), _, _) => Err(error.into_failure()),
        }
    }

    async fn sql(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &["operation", "sql", "parameters", "mode"])?;
        let sql = text(command, "sql")?;
        if sql.contains('\0') || sql.contains(';') {
            return Err(failure(
                "FYLO SQL takes one statement without control characters.",
            ));
        }
        let parameters = Self::sql_parameters(command)?;
        let bound = SqlStatement::bound(sql, parameters.len())?;
        let answer = self
            .call("executeSQL", Self::sql_request(&bound, parameters))
            .await
            .map_err(FyloError::into_failure)?;
        Self::sql_answer(text(command, "mode")?, answer)
    }

    fn sql_request(sql: &str, parameters: &[Value]) -> Value {
        if parameters.is_empty() {
            return json!({ "sql": sql });
        }
        json!({ "sql": sql, "parameters": parameters })
    }

    fn sql_parameters(command: &Value) -> Result<&Vec<Value>, Failure> {
        let parameters = command["parameters"]
            .as_array()
            .filter(|parameters| parameters.len() <= Self::MAX_SQL_PARAMETERS)
            .ok_or_else(|| failure("SQL requires a bounded parameter array."))?;
        for value in parameters {
            scalar(value)?;
        }
        Ok(parameters)
    }

    fn sql_answer(mode: &str, answer: Value) -> Result<Value, Failure> {
        match (mode, &answer) {
            ("rows", Value::Array(_)) => Ok(Value::Array(StoreRows::found(answer)?)),
            ("rows", _) => Ok(json!([])),
            (_, Value::Number(affected)) => Ok(json!({ "affectedRows": affected })),
            (_, Value::String(id)) => Ok(json!({ "affectedRows": 1, "id": id })),
            _ => Ok(json!({ "affectedRows": 0 })),
        }
    }

    async fn call_joined(
        &self,
        joined: Option<&str>,
        operation: &str,
        mut fields: Value,
    ) -> Result<Value, FyloError> {
        if let Some(handle) = joined {
            fields["transaction"] = json!(handle);
        }
        self.call(operation, fields).await
    }

    pub(super) async fn close(&self) {
        self.closed.store(true, Ordering::Release);
        let closed = std::time::Instant::now();
        self.live_input.close();
        let slot = tokio::time::timeout(FyloProcess::SHUTDOWN_GRACE, self.process.lock()).await;
        let process = slot.ok().and_then(|mut slot| slot.take());
        self.transactions.close();
        OptionFuture::from(process.map(|process| process.close(closed))).await;
    }

    pub(super) fn operation_timeout(&self) -> Duration {
        self.operation_timeout
    }

    pub(super) fn deadline_failure(&self) -> Option<Failure> {
        let operation = self.busy.name()?;
        Some(Self::interrupted_failure(&operation))
    }

    fn interrupted_failure(operation: &str) -> Failure {
        let message = format!(
            "Managed storage operation deadline exceeded during FYLO {operation}; the store-time budget is spent and writes may have committed. Tachyon stops that FYLO process, and the next call waits until it has exited."
        );
        Failure::one(diagnostic(2160, message, None, None))
    }

    async fn call(&self, operation: &str, fields: Value) -> Result<Value, FyloError> {
        let mut answers = self.call_together(vec![(operation, fields)]).await?;
        Ok(answers.pop().unwrap_or(Value::Null))
    }

    async fn patch(&self, collection: &str, id: &str, fields: Map<String, Value>) -> PatchAnswer {
        let answer = self.patches.add(collection, id, fields);
        let mut guard = StoreCall::waiting_behind(self.process.lock()).await;
        self.write_waiting_patches(&mut guard).await;
        drop(guard);
        PatchQueue::received(answer)
    }

    async fn write_waiting_patches(&self, guard: &mut Option<FyloProcess>) {
        for combined in self.patches.take_combined() {
            let answer = self.call_holding(guard, combined.requests()).await;
            combined.answer(&answer);
        }
    }

    async fn call_together(&self, requests: Vec<(&str, Value)>) -> Result<Vec<Value>, FyloError> {
        let mut guard = StoreCall::waiting_behind(self.process.lock()).await;
        self.call_holding(&mut guard, requests).await
    }

    async fn call_holding(
        &self,
        guard: &mut Option<FyloProcess>,
        requests: Vec<(&str, Value)>,
    ) -> Result<Vec<Value>, FyloError> {
        self.refuse_when_closed()?;
        let process = self.ready_process(guard).await?;
        self.send_each(process, requests).await
    }

    fn refuse_without_time(startup: bool) -> Result<(), FyloError> {
        let left = StoreCall::time_left(startup).unwrap_or(Duration::MAX);
        if left < Self::MIN_SEND_TIME {
            return Err(FyloError {
                code: String::from("ENOTSENT"),
            });
        }
        Ok(())
    }

    fn refuse_when_closed(&self) -> Result<(), FyloError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(FyloError::protocol("FYLO datasource is closed."));
        }
        Ok(())
    }

    async fn send_each(
        &self,
        process: &mut FyloProcess,
        requests: Vec<(&str, Value)>,
    ) -> Result<Vec<Value>, FyloError> {
        let mut answers = Vec::with_capacity(requests.len());
        for (operation, mut fields) in requests {
            fields["op"] = json!(operation);
            let target = StartupRequests::target_of(&fields);
            let startup = self.first_requests.is_first(&target);
            Self::refuse_without_time(startup)?;
            self.busy.mark(operation);
            let answer = process.request(operation, &fields, startup).await;
            self.busy.clear();
            self.first_requests.record(target, &answer);
            answers.push(answer?);
        }
        Ok(answers)
    }

    async fn ready_process<'slot>(
        &self,
        slot: &'slot mut Option<FyloProcess>,
    ) -> Result<&'slot mut FyloProcess, FyloError> {
        Self::stop_interrupted(slot).await;
        self.busy.clear();
        self.spawn_when_empty(slot)?;
        let process = slot
            .as_mut()
            .ok_or_else(|| FyloError::protocol("FYLO subprocess is unavailable."))?;
        self.live_input.follow(process.input());
        Ok(process)
    }

    fn spawn_when_empty(&self, slot: &mut Option<FyloProcess>) -> Result<(), FyloError> {
        if slot.is_none() {
            *slot = Some(FyloProcess::spawn(&self.root, self.namespace.as_deref())?);
            self.first_requests.restart();
        }
        Ok(())
    }

    async fn stop_interrupted(slot: &mut Option<FyloProcess>) {
        let Some(process) = slot.as_mut().filter(|process| process.must_stop()) else {
            return;
        };
        StoreCall::startup(process.wait_for_exit()).await;
        *slot = None;
    }
}

#[derive(Clone)]
struct FyloError {
    code: String,
}

impl FyloError {
    const NATIVE_NOT_FOUND: &'static str = "ENATIVE_NOT_FOUND";

    fn protocol(_: &'static str) -> Self {
        Self {
            code: String::from("EPROTOCOL"),
        }
    }

    fn broke_process(&self) -> bool {
        self.code == "EPROTOCOL"
    }

    fn root_locked(&self) -> bool {
        self.code == "EROOTLOCKED"
    }

    fn missing(&self) -> bool {
        matches!(self.code.as_str(), "EINVALIDDOCID" | "ENATIVE_NOT_FOUND")
    }

    fn transaction_gone(&self) -> bool {
        matches!(self.code.as_str(), "ETXNUNKNOWN" | "ETXNEXPIRED")
    }

    fn read_found_no_collection(&self) -> bool {
        self.code == Self::NATIVE_NOT_FOUND
    }

    fn collection_missing(&self) -> bool {
        matches!(
            self.code.as_str(),
            "ENOTFOUND" | "ECOLLECTION_NOT_FOUND" | "ENATIVE_UNSAFE_PATH" | "ENATIVE_NOT_FOUND"
        )
    }

    fn into_failure(self) -> Failure {
        match self.code.as_str() {
            "EINVALIDDOCID" | "ENATIVE_NOT_FOUND" => failure("FYLO document does not exist."),
            "ENOTFOUND" | "ECOLLECTION_NOT_FOUND" | "ENATIVE_UNSAFE_PATH" => {
                failure("FYLO collection does not exist; create it before use.")
            }
            "EPROTOCOL" => {
                failure("FYLO subprocess failed; the datasource reconnects on the next call.")
            }
            "ENOTSENT" => failure(
                "Managed storage operation deadline exceeded before the FYLO request was sent; the store-time budget is spent and nothing was written.",
            ),
            "ECURSOR" => failure(
                "The scroll cursor expired or belongs to another query; start again from the first page.",
            ),
            "EQUERY_INVALID" => failure(
                "FYLO refused the query: an unknown operator, a value of the wrong type, or an object as a sort key.",
            ),
            "EUNSUPPORTEDOP" => failure(
                "This FYLO does not have the Store operation Tachyon sent; install the FYLO version that Tachyon pins.",
            ),
            "ETXNLAYOUT" => failure(
                "This FYLO collection's layout cannot hold a transaction; write without @AllOrNothing.",
            ),
            "EQUERY_INPUT_SIZE" => failure(store_query::StoreQuery::TOO_LARGE),
            "EQUERYLIMIT" | "EPAGESIZE" => failure(
                "The query passes FYLO's Store limits: 8 predicates, 4 sort fields, 100 In values and 64 KiB of query.",
            ),
            "ETXNUNKNOWN" | "ETXNEXPIRED" => failure(
                "The FYLO transaction expired or its FYLO process stopped; nothing in it was written. Start a new transaction.",
            ),
            "ETXNLIMIT" => failure(
                "The FYLO transaction passed its bounds (1,000 writes or 8 MiB); FYLO rolled it back.",
            ),
            "ETXNSCOPE" => failure(
                "A FYLO transaction covers the writes of one collection; FYLO rolled it back.",
            ),
            "ETXNBUSY" => {
                failure("Another transaction holds this FYLO collection; try the write again.")
            }
            code => Failure::one(crate::failure::diagnostic(
                2160,
                format!("FYLO operation failed ({code})."),
                None,
                None,
            )),
        }
    }
}

fn identifier(name: &str) -> Result<&str, Failure> {
    if !is_simple_identifier(name) {
        return Err(failure("Storage identifiers require simple ASCII names."));
    }
    Ok(name)
}

fn document_id(value: &Value) -> Result<&str, Failure> {
    value
        .as_str()
        .filter(|id| {
            !id.is_empty() && id.len() <= 64 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
        .ok_or_else(|| failure("FYLO identifiers are the TTIDs it assigns."))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::FyloStore;

    #[test]
    fn a_statement_without_parameters_sends_no_parameters_field() {
        let request = FyloStore::sql_request("SELECT * FROM products", &[]);
        assert_eq!(request, json!({"sql": "SELECT * FROM products"}));
    }

    #[test]
    fn a_statement_with_parameters_sends_them_as_a_list() {
        let request =
            FyloStore::sql_request("SELECT * FROM products WHERE name = ?1", &[json!("rope")]);
        assert_eq!(
            request,
            json!({"sql": "SELECT * FROM products WHERE name = ?1", "parameters": ["rope"]})
        );
    }
}
