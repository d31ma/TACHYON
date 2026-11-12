mod call_lease;
mod collection_name;
mod document_commands;
mod frame_answer;
mod fylo_lease;
#[cfg(any(unix, test))]
mod mount_entry;
mod network_root;
mod operation_deadline;
mod patch_queue;
mod queue_commands;
mod queue_name;
mod root_lock_wait;
mod scroll_page;
mod scroll_request;
mod scroll_snapshot;
mod scroll_snapshots;
mod sql_statement;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde_json::{Map, Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

use super::dialect::{
    is_simple_identifier, page_with_query_order, reject_count_page_or_order,
    sort_field_and_direction,
};
use super::{ManagedStorage, failure, object_keys, scalar, text};
use crate::Failure;
use crate::sorted_json::SortedJson;
use call_lease::CallLease;
pub(super) use collection_name::CollectionName;
use document_commands::DocumentCommands;
use frame_answer::FrameAnswer;
use fylo_lease::FyloLease;
use operation_deadline::OperationDeadline;
use patch_queue::{PatchAnswer, PatchQueue};
use queue_commands::QueueCommands;
use root_lock_wait::RootLockWait;
use scroll_snapshots::ScrollSnapshots;
pub(super) use sql_statement::SqlStatement;

pub(super) struct FyloStore {
    root: PathBuf,
    lease: FyloLease,
    process: Mutex<Option<FyloProcess>>,
    patches: PatchQueue,
    closed: AtomicBool,
    scrolls: Mutex<ScrollSnapshots>,
}

struct FyloProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl FyloStore {
    const MAX_SCAN: usize = 10_000;

    pub(super) async fn connect(settings: &Value) -> Result<Self, Failure> {
        object_keys(settings, &["driver", "url", "lease"])?;
        let lease = FyloLease::from_settings(settings)?;
        let raw = text(settings, "url")?;
        let path = Self::root_path(raw)
            .ok_or_else(|| failure("FYLO datasource URL is fylo:///absolute/root."))?;
        if path.contains('\0') || path.contains("..") {
            return Err(failure("FYLO root path contains an unsafe segment."));
        }
        let root = std::fs::canonicalize(Path::new(path))
            .map(Self::without_verbatim_prefix)
            .map_err(|_| failure("FYLO root directory must already exist."))?;
        if !root.is_dir() {
            return Err(failure("FYLO root must be a directory."));
        }
        let process = Self::first_process(lease, &root).await?;
        Ok(Self {
            root,
            lease,
            process: Mutex::new(process),
            patches: PatchQueue::new(),
            closed: AtomicBool::new(false),
            scrolls: Mutex::new(ScrollSnapshots::new()),
        })
    }

    async fn first_process(lease: FyloLease, root: &Path) -> Result<Option<FyloProcess>, Failure> {
        match lease {
            FyloLease::Server => Ok(Some(FyloProcess::started(root).await?)),
            FyloLease::Call => {
                CallLease::new(root).prepare().await?;
                Ok(None)
            }
        }
    }

    fn root_path(raw: &str) -> Option<&str> {
        let path = raw
            .strip_prefix("fylo://")
            .filter(|rest| rest.starts_with('/'))?;
        let windows_drive = [cfg!(windows), path.get(2..3) == Some(":")] == [true, true];
        path.get(usize::from(windows_drive)..)
    }

    fn without_verbatim_prefix(root: PathBuf) -> PathBuf {
        let drive_path = root
            .to_str()
            .and_then(|text| text.strip_prefix(r"\\?\"))
            .filter(|rest| rest.get(1..2) == Some(":"))
            .map(PathBuf::from);
        drive_path.unwrap_or(root)
    }

    pub(super) async fn execute(
        &self,
        command: &Value,
        deadline: Instant,
    ) -> Result<Value, Failure> {
        OperationDeadline::scope(deadline, self.run(command)).await
    }

    async fn run(&self, command: &Value) -> Result<Value, Failure> {
        let operation = text(command, "operation")?;
        match operation {
            "sql" => return self.sql(command).await,
            "document" => return DocumentCommands::new(self).execute(command).await,
            "queue" => return QueueCommands::new(self).execute(command).await,
            _ => {}
        }
        object_keys(command, &ManagedStorage::REPOSITORY_COMMAND_KEYS)?;
        let collection = CollectionName::checked(text(command, "table")?)?;
        let id_field = identifier(text(command, "idField")?)?;
        match operation {
            "save" => self.save(command, collection, id_field).await,
            "delete" => self.delete(command, collection).await,
            "find" => {
                let page = command.get("page").cloned().unwrap_or(Value::Null);
                self.page(collection, id_field, Vec::new(), &page).await
            }
            "query" => self.query(command, collection, id_field).await,
            _ => Err(failure("Unsupported managed storage operation.")),
        }
    }

    pub(super) async fn close(&self) {
        self.closed.store(true, Ordering::Release);
        if let Some(mut process) = self.process.lock().await.take() {
            let _ = process.stdin.shutdown().await;
            let _ = process.child.kill().await;
        }
    }

    async fn delete(&self, command: &Value, collection: &str) -> Result<Value, Failure> {
        let Some(id) = command.get("id") else {
            return Err(failure("Store delete requires an identifier."));
        };
        let id = document_id(id)?;
        let answer = self
            .call("delDoc", json!({ "collection": collection, "id": id }))
            .await;
        match answer {
            Ok(_) => Ok(json!(true)),
            Err(error) if error.missing() => Ok(json!(false)),
            Err(error) => Err(error.into_failure()),
        }
    }

    async fn save(
        &self,
        command: &Value,
        collection: &str,
        id_field: &str,
    ) -> Result<Value, Failure> {
        let value = command
            .get("value")
            .ok_or_else(|| failure("Store save requires an entity."))?;
        let data = value
            .as_object()
            .filter(|data| !data.is_empty() && data.len() <= 64)
            .ok_or_else(|| failure("Store save requires a bounded entity object."))?;
        let mut document = data.clone();
        let existing = document.remove(id_field);
        match existing {
            None | Some(Value::Null) => self.insert_document(collection, id_field, document).await,
            Some(id) => {
                let id = document_id(&id)?;
                self.update_document(collection, id_field, id, document)
                    .await
            }
        }
    }

    async fn insert_document(
        &self,
        collection: &str,
        id_field: &str,
        document: Map<String, Value>,
    ) -> Result<Value, Failure> {
        let request = json!({ "collection": collection, "data": Value::Object(document.clone()) });
        let id = self
            .call("putData", request)
            .await
            .map_err(FyloError::into_failure)?;
        let mut saved = document;
        saved.insert(id_field.to_owned(), id);
        Ok(Value::Object(saved))
    }

    async fn update_document(
        &self,
        collection: &str,
        id_field: &str,
        id: &str,
        document: Map<String, Value>,
    ) -> Result<Value, Failure> {
        let request =
            json!({ "collection": collection, "id": id, "newDoc": Value::Object(document) });
        self.call("patchDoc", request).await.map_err(|error| {
            if error.missing() {
                failure("FYLO assigns identifiers; save a new entity without one.")
            } else {
                error.into_failure()
            }
        })?;
        let fetched = self
            .call("getDoc", json!({ "collection": collection, "id": id }))
            .await
            .map_err(FyloError::into_failure)?;
        let Some(saved) = rows_of(fetched, id_field)?.into_iter().next() else {
            return Err(failure("FYLO did not return the saved document."));
        };
        Ok(saved)
    }

    async fn query(
        &self,
        command: &Value,
        collection: &str,
        id_field: &str,
    ) -> Result<Value, Failure> {
        let query = &command["query"];
        object_keys(query, &["kind", "where", "orderBy", "page"])?;
        let kind = text(query, "kind")?;
        let predicates = query["where"]
            .as_array()
            .filter(|items| items.len() <= 8)
            .ok_or_else(|| failure("Store query requires bounded predicates."))?;
        let (operators, in_predicates) = predicates_of(predicates)?;
        let alternatives = query_alternatives(operators, &in_predicates);
        if alternatives.is_empty() {
            return empty_query_result(kind);
        }
        let filter = alternatives_filter(&alternatives);
        let extra = in_predicates.into_iter().skip(1).collect::<Vec<_>>();
        if kind == "find" {
            let page = page_with_query_order(query)?;
            return self
                .page_filtered(collection, id_field, filter, extra, &page)
                .await;
        }
        reject_count_page_or_order(query)?;
        let limit = if kind == "exists" { 1 } else { Self::MAX_SCAN };
        let rows = self
            .scan(collection, id_field, filter, &extra, limit)
            .await?;
        match kind {
            "count" => Ok(json!(rows.len())),
            "exists" => Ok(json!(!rows.is_empty())),
            "delete" => self.delete_rows(collection, id_field, &rows).await,
            _ => Err(failure("Unsupported repository query kind.")),
        }
    }

    async fn delete_rows(
        &self,
        collection: &str,
        id_field: &str,
        rows: &[Value],
    ) -> Result<Value, Failure> {
        let mut deleted = 0_usize;
        for row in rows {
            let command = json!({ "id": row.get(id_field).cloned().unwrap_or(Value::Null) });
            let answer = self.delete(&command, collection).await?;
            deleted += usize::from(answer == json!(true));
        }
        Ok(json!(deleted))
    }

    async fn page(
        &self,
        collection: &str,
        id_field: &str,
        extra: Vec<(String, Vec<Value>)>,
        page: &Value,
    ) -> Result<Value, Failure> {
        self.page_filtered(collection, id_field, json!({}), extra, page)
            .await
    }

    async fn page_filtered(
        &self,
        collection: &str,
        id_field: &str,
        filter: Value,
        extra: Vec<(String, Vec<Value>)>,
        page: &Value,
    ) -> Result<Value, Failure> {
        let page = if page.is_null() {
            json!({})
        } else {
            page.clone()
        };
        object_keys(&page, &["page", "size", "sort", "offset"])?;
        let (offset, size) = page_offset_and_size(&page)?;
        let sort = sort_keys(&page, id_field)?;
        if offset + size > Self::MAX_SCAN {
            return Err(failure(
                "FYLO pages are bounded to the first 10,000 matching documents.",
            ));
        }
        let mut rows = self
            .scan(collection, id_field, filter, &extra, Self::MAX_SCAN)
            .await?;
        rows.sort_by(|left, right| compare_by_sort_keys(left, right, &sort));
        Ok(Value::Array(
            rows.into_iter().skip(offset).take(size).collect(),
        ))
    }

    async fn scan(
        &self,
        collection: &str,
        id_field: &str,
        filter: Value,
        extra: &[(String, Vec<Value>)],
        limit: usize,
    ) -> Result<Vec<Value>, Failure> {
        let mut filter = filter;
        filter["$limit"] = json!(limit.min(Self::MAX_SCAN));
        let answer = self
            .call(
                "findDocs",
                json!({ "collection": collection, "query": filter }),
            )
            .await
            .map_err(FyloError::into_failure)?;
        let mut rows = Vec::new();
        for row in rows_of(answer, id_field)? {
            if matches_every_in_predicate(&row, extra) {
                rows.push(row);
            }
        }
        if rows.len() > Self::MAX_SCAN {
            return Err(failure(
                "FYLO result exceeds the scan bound; narrow the query.",
            ));
        }
        Ok(rows)
    }

    async fn sql(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &["operation", "sql", "parameters", "mode"])?;
        let sql = text(command, "sql")?;
        if sql.contains('\0') || sql.contains(';') {
            return Err(failure(
                "FYLO SQL takes one statement without control characters.",
            ));
        }
        let parameters = command["parameters"]
            .as_array()
            .filter(|parameters| parameters.len() <= 64)
            .ok_or_else(|| failure("SQL requires a bounded parameter array."))?;
        for value in parameters {
            scalar(value)?;
        }
        let rendered = SqlStatement::rendered(sql, parameters)?;
        let answer = self
            .call("executeSQL", json!({ "sql": rendered }))
            .await
            .map_err(FyloError::into_failure)?;
        let mode = text(command, "mode")?;
        match (mode, &answer) {
            ("rows", Value::Object(_)) => Ok(Value::Array(rows_of(answer, "id")?)),
            ("rows", _) => Ok(json!([])),
            (_, Value::Number(affected)) => Ok(json!({ "affectedRows": affected })),
            (_, Value::String(id)) => Ok(json!({ "affectedRows": 1, "id": id })),
            _ => Ok(json!({ "affectedRows": 0 })),
        }
    }

    async fn call(&self, operation: &str, fields: Value) -> Result<Value, FyloError> {
        let mut answers = self.call_together(vec![(operation, fields)]).await?;
        Ok(answers.pop().unwrap_or(Value::Null))
    }

    async fn patch(&self, collection: &str, id: &str, fields: Map<String, Value>) -> PatchAnswer {
        let answer = self.patches.add(collection, id, fields);
        let mut guard = self.process.lock().await;
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
        let mut guard = self.process.lock().await;
        self.call_holding(&mut guard, requests).await
    }

    async fn call_holding(
        &self,
        guard: &mut Option<FyloProcess>,
        requests: Vec<(&str, Value)>,
    ) -> Result<Vec<Value>, FyloError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(FyloError::protocol("FYLO datasource is closed."));
        }
        if self.lease == FyloLease::Call {
            return self.call_with_lease(requests).await;
        }
        let mut process = match guard.take() {
            Some(process) => process,
            None => FyloProcess::spawn(&self.root)?,
        };
        let mut answers = Vec::with_capacity(requests.len());
        for (operation, mut fields) in requests {
            fields["op"] = json!(operation);
            match process.request_waiting(fields).await {
                Ok(answer) => answers.push(answer),
                Err(error) if error.code == "EPROTOCOL" => return Err(error),
                Err(error) => {
                    *guard = Some(process);
                    return Err(error);
                }
            }
        }
        *guard = Some(process);
        Ok(answers)
    }

    async fn call_with_lease(&self, requests: Vec<(&str, Value)>) -> Result<Vec<Value>, FyloError> {
        let lease = CallLease::new(&self.root);
        let mut answers = Vec::with_capacity(requests.len());
        for (operation, fields) in requests {
            answers.push(lease.answer(operation, fields).await?);
        }
        Ok(answers)
    }
}

type InPredicate = (String, Vec<Value>);

fn predicates_of(predicates: &[Value]) -> Result<(Map<String, Value>, Vec<InPredicate>), Failure> {
    let mut operators = Map::new();
    let mut in_predicates = Vec::new();
    for predicate in predicates {
        object_keys(predicate, &["field", "operator", "value"])?;
        let field = identifier(text(predicate, "field")?)?;
        match text(predicate, "operator")? {
            "isNull" => {
                operators.insert(field.to_owned(), json!({ "$eq": null }));
            }
            "isNotNull" => {
                operators.insert(field.to_owned(), json!({ "$ne": null }));
            }
            "in" => {
                let values = in_predicate_values(predicate)?;
                in_predicates.push((field.to_owned(), values.clone()));
            }
            operator => {
                let condition = comparison_condition(predicate, operator)?;
                operators.insert(field.to_owned(), condition);
            }
        }
    }
    Ok((operators, in_predicates))
}

fn in_predicate_values(predicate: &Value) -> Result<&Vec<Value>, Failure> {
    let values = predicate["value"]
        .as_array()
        .filter(|values| values.len() <= 100)
        .ok_or_else(|| failure("In predicate requires a bounded array."))?;
    for value in values {
        scalar(value)?;
    }
    Ok(values)
}

fn comparison_condition(predicate: &Value, operator: &str) -> Result<Value, Failure> {
    let key = match operator {
        "eq" => "$eq",
        "ne" => "$ne",
        "lt" => "$lt",
        "lte" => "$lte",
        "gt" => "$gt",
        "gte" => "$gte",
        _ => return Err(failure("Unsupported repository comparison.")),
    };
    let value = predicate
        .get("value")
        .ok_or_else(|| failure("Comparison predicate requires a value."))?;
    scalar(value)?;
    Ok(json!({ key: value }))
}

fn query_alternatives(
    operators: Map<String, Value>,
    in_predicates: &[InPredicate],
) -> Vec<Map<String, Value>> {
    let Some((field, values)) = in_predicates.first() else {
        return vec![operators];
    };
    let mut alternatives = Vec::new();
    for value in values {
        let mut alternative = operators.clone();
        alternative.insert(field.clone(), json!({ "$eq": value }));
        alternatives.push(alternative);
    }
    alternatives
}

fn alternatives_filter(alternatives: &[Map<String, Value>]) -> Value {
    if alternatives.len() == 1 && alternatives[0].is_empty() {
        return json!({});
    }
    json!({ "$ops": alternatives })
}

fn empty_query_result(kind: &str) -> Result<Value, Failure> {
    match kind {
        "find" => Ok(json!([])),
        "count" | "delete" => Ok(json!(0)),
        "exists" => Ok(json!(false)),
        _ => Err(failure("Unsupported repository query kind.")),
    }
}

fn page_offset_and_size(page: &Value) -> Result<(usize, usize), Failure> {
    let index = page
        .get("page")
        .map_or(Some(0), Value::as_u64)
        .filter(|index| *index <= 1_000_000)
        .ok_or_else(|| failure("Invalid repository page."))?;
    let size = page
        .get("size")
        .map_or(Some(20), Value::as_u64)
        .filter(|size| *size > 0 && *size <= ManagedStorage::MAX_ROWS as u64)
        .ok_or_else(|| failure("Invalid repository page size."))?;
    let offset_mismatch = page
        .get("offset")
        .is_some_and(|offset| offset.as_u64() != Some(index * size));
    if offset_mismatch {
        return Err(failure("Invalid repository page offset."));
    }
    let offset =
        usize::try_from(index * size).map_err(|_| failure("Invalid repository page offset."))?;
    let size = usize::try_from(size).map_err(|_| failure("Invalid repository page size."))?;
    Ok((offset, size))
}

fn sort_keys(page: &Value, id_field: &str) -> Result<Vec<(String, bool)>, Failure> {
    let mut sort = Vec::new();
    if let Some(items) = page.get("sort") {
        let items = items
            .as_array()
            .filter(|sort| sort.len() <= 5)
            .ok_or_else(|| failure("Invalid repository sorting."))?;
        for item in items {
            let (field, direction) = sort_field_and_direction(item)?;
            let descending = match direction {
                "asc" => false,
                "desc" => true,
                _ => return Err(failure("Invalid repository sort direction.")),
            };
            sort.push((identifier(field)?.to_owned(), descending));
        }
    }
    if sort.is_empty() {
        sort.push((id_field.to_owned(), false));
    }
    Ok(sort)
}

fn compare_by_sort_keys(
    left: &Value,
    right: &Value,
    sort: &[(String, bool)],
) -> std::cmp::Ordering {
    for (field, descending) in sort {
        let ordering = compare(&left[field], &right[field]);
        if ordering == std::cmp::Ordering::Equal {
            continue;
        }
        if *descending {
            return ordering.reverse();
        }
        return ordering;
    }
    std::cmp::Ordering::Equal
}

fn matches_every_in_predicate(row: &Value, in_predicates: &[InPredicate]) -> bool {
    for (field, values) in in_predicates {
        if !values.iter().any(|value| row[field] == *value) {
            return false;
        }
    }
    true
}

impl FyloProcess {
    const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

    async fn started(root: &Path) -> Result<Self, Failure> {
        let mut process = Self::spawn(root).map_err(FyloError::into_failure)?;
        let identity = process
            .request(&json!({ "op": "handshake" }))
            .await
            .map_err(FyloError::into_failure)?;
        Self::supported_protocol(&identity)?;
        Ok(process)
    }

    fn supported_protocol(identity: &Value) -> Result<(), Failure> {
        if identity["protocolVersion"] != json!(1) {
            return Err(failure(
                "FYLO speaks an unsupported machine protocol version.",
            ));
        }
        Ok(())
    }

    fn spawn(root: &Path) -> Result<Self, FyloError> {
        let mut child = tokio::process::Command::new("fylo")
            .args(["exec", "--loop", "--root"])
            .arg(root)
            .args(["--max-request-bytes", "1048576", "--max-response-bytes", "8388608"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                FyloError::protocol(
                    "The fylo binary is not on PATH; install FYLO (fylo.del.ma) to use this datasource.",
                )
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdin is unavailable."))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdout is unavailable."))?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    async fn request_waiting(&mut self, request: Value) -> Result<Value, FyloError> {
        let mut wait = RootLockWait::new();
        let mut answer = self.request(&request).await;
        while Self::root_was_busy(&answer, &mut wait).await {
            answer = self.request(&request).await;
        }
        answer
    }

    async fn root_was_busy(answer: &Result<Value, FyloError>, wait: &mut RootLockWait) -> bool {
        let root_locked = answer.as_ref().err().is_some_and(FyloError::root_locked);
        root_locked && wait.pause().await
    }

    async fn request(&mut self, request: &Value) -> Result<Value, FyloError> {
        let mut line = SortedJson::text(request);
        if line.len() > 1024 * 1024 {
            return Err(FyloError::protocol("FYLO request exceeds 1 MiB."));
        }
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|_| FyloError::protocol("FYLO subprocess is not accepting requests."))?;
        let mut answer = String::new();
        let mut reader =
            tokio::io::AsyncReadExt::take(&mut self.stdout, Self::MAX_RESPONSE_BYTES as u64 + 1);
        let read = reader
            .read_line(&mut answer)
            .await
            .map_err(|_| FyloError::protocol("FYLO subprocess closed its output."))?;
        if read == 0 {
            return Err(FyloError::protocol("FYLO subprocess exited."));
        }
        if answer.len() > Self::MAX_RESPONSE_BYTES {
            return Err(FyloError::protocol("FYLO response exceeds 8 MiB."));
        }
        FrameAnswer::from_line(&answer)
    }
}

#[derive(Clone)]
struct FyloError {
    code: String,
}

impl FyloError {
    fn protocol(_: &'static str) -> Self {
        Self {
            code: String::from("EPROTOCOL"),
        }
    }

    fn coded(code: &str) -> Self {
        Self {
            code: code.to_owned(),
        }
    }

    fn root_locked(&self) -> bool {
        self.code == "EROOTLOCKED"
    }

    fn missing(&self) -> bool {
        matches!(self.code.as_str(), "EINVALIDDOCID" | "ENATIVE_NOT_FOUND")
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
            "ENOFRAME" => failure(
                "FYLO exited without an answer frame (ENOFRAME): a FYLO host setting or the fylo version is wrong.",
            ),
            code => Failure::one(crate::failure::diagnostic(
                2160,
                format!("FYLO operation failed ({code})."),
                None,
                None,
            )),
        }
    }
}

fn rows_of(answer: Value, id_field: &str) -> Result<Vec<Value>, Failure> {
    let items = match answer {
        Value::Object(map) if map.contains_key("items") && map.contains_key("page") => {
            map["items"].clone()
        }
        other => other,
    };
    let Value::Object(items) = items else {
        return Err(failure("FYLO returned an unexpected document shape."));
    };
    let mut rows = Vec::with_capacity(items.len());
    let mut bytes = 0;
    for (id, document) in SortedJson::owned_entries(items) {
        let Value::Object(mut document) = document else {
            continue;
        };
        document.insert(id_field.to_owned(), Value::String(id));
        let row = Value::Object(document);
        bytes += serde_json::to_vec(&row).map_or(0, |encoded| encoded.len());
        if bytes > ManagedStorage::MAX_BYTES {
            return Err(failure(
                "FYLO result exceeds the byte limit; paginate the query.",
            ));
        }
        rows.push(row);
    }
    Ok(rows)
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

fn compare(left: &Value, right: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .as_f64()
            .partial_cmp(&right.as_f64())
            .unwrap_or(Ordering::Equal),
        (Value::String(left), Value::String(right)) => left.cmp(right),
        (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Less,
        (_, Value::Null) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}
