use std::path::{Path, PathBuf};
use std::process::Stdio;

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

pub(super) struct FyloStore {
    root: PathBuf,
    process: Mutex<Option<FyloProcess>>,
}

struct FyloProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl FyloStore {
    const MAX_SCAN: usize = 10_000;

    pub(super) async fn connect(settings: &Value) -> Result<Self, Failure> {
        object_keys(settings, &["driver", "url"])?;
        let raw = text(settings, "url")?;
        let path = raw
            .strip_prefix("fylo://")
            .filter(|rest| rest.starts_with('/'))
            .ok_or_else(|| failure("FYLO datasource URL is fylo:///absolute/root."))?;
        if path.contains('\0') || path.contains("..") {
            return Err(failure("FYLO root path contains an unsafe segment."));
        }
        let root = std::fs::canonicalize(Path::new(path))
            .map_err(|_| failure("FYLO root directory must already exist."))?;
        if !root.is_dir() {
            return Err(failure("FYLO root must be a directory."));
        }
        let mut process = FyloProcess::spawn(&root).map_err(FyloError::into_failure)?;
        let identity = process
            .request(json!({ "op": "handshake" }))
            .await
            .map_err(FyloError::into_failure)?;
        if identity["protocolVersion"] != json!(1) {
            return Err(failure(
                "FYLO speaks an unsupported machine protocol version.",
            ));
        }
        Ok(Self {
            root,
            process: Mutex::new(Some(process)),
        })
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        let operation = text(command, "operation")?;
        if operation == "sql" {
            return self.sql(command).await;
        }
        object_keys(command, &ManagedStorage::REPOSITORY_COMMAND_KEYS)?;
        let collection = identifier(text(command, "table")?)?;
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
        let rendered = render_sql_parameters(sql, parameters)?;
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

    async fn call(&self, operation: &str, mut fields: Value) -> Result<Value, FyloError> {
        fields["op"] = json!(operation);
        let mut guard = self.process.lock().await;
        let process = guard
            .as_mut()
            .ok_or_else(|| FyloError::protocol("FYLO datasource is closed."))?;
        let answer = process.request(fields).await;
        let is_protocol_error = answer
            .as_ref()
            .is_err_and(|error| error.code == "EPROTOCOL");
        if is_protocol_error && let Ok(fresh) = FyloProcess::spawn(&self.root) {
            *process = fresh;
        }
        answer
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

fn render_sql_parameters(sql: &str, parameters: &[Value]) -> Result<String, Failure> {
    let mut rendered = String::with_capacity(sql.len());
    let mut rest = sql;
    while let Some(at) = rest.find('$') {
        rendered.push_str(&rest[..at]);
        let after_dollar = &rest[at + 1..];
        let digits = after_dollar.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return Err(failure("FYLO SQL parameters are $1, $2, ..."));
        }
        let index = after_dollar[..digits]
            .parse::<usize>()
            .ok()
            .filter(|index| *index >= 1 && *index <= parameters.len())
            .ok_or_else(|| failure("SQL parameter index is out of range."))?;
        rendered.push_str(&literal(&parameters[index - 1]));
        rest = &after_dollar[digits..];
    }
    rendered.push_str(rest);
    Ok(rendered)
}

impl FyloProcess {
    const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

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

    async fn request(&mut self, request: Value) -> Result<Value, FyloError> {
        let mut line = serde_json::to_string(&request)
            .map_err(|_| FyloError::protocol("FYLO request could not be encoded."))?;
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
        let envelope: Value = serde_json::from_str(answer.trim_end())
            .map_err(|_| FyloError::protocol("FYLO returned a malformed frame."))?;
        if envelope["ok"] == json!(true) {
            return Ok(envelope["result"].clone());
        }
        Err(FyloError {
            code: envelope["error"]["code"]
                .as_str()
                .unwrap_or("EUNKNOWN")
                .to_owned(),
        })
    }
}

struct FyloError {
    code: String,
}

impl FyloError {
    fn protocol(_: &'static str) -> Self {
        Self {
            code: String::from("EPROTOCOL"),
        }
    }

    fn missing(&self) -> bool {
        matches!(self.code.as_str(), "EINVALIDDOCID" | "ENATIVE_NOT_FOUND")
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
    for (id, document) in items {
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

fn literal(value: &Value) -> String {
    match value {
        Value::Bool(value) => String::from(if *value { "true" } else { "false" }),
        Value::Number(value) => value.to_string(),
        Value::String(value) => format!("'{}'", value.replace('\'', "''")),
        _ => String::from("NULL"),
    }
}
