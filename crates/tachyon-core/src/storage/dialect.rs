use std::fmt::Write as _;

use serde_json::{Value, json};

use super::{ManagedStorage, failure, object_keys, scalar, text};
use crate::Failure;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Dialect {
    Postgres,
    Mysql,
    Sqlite,
    SqlServer,
}

pub(super) struct Statement {
    pub sql: String,
    pub parameters: Vec<Value>,
    pub result: ResultKind,
    pub isolated: bool,
}

pub(super) enum ResultKind {
    Rows,
    Count,
    Exists,
    Changed,
    Saved(Value),
    Affected,
    Deleted,
}

impl ResultKind {
    pub(super) fn returns_rows(&self) -> bool {
        self.reads_rows()
    }

    fn reads_rows(&self) -> bool {
        matches!(self, Self::Rows | Self::Count | Self::Exists)
    }
}

impl Dialect {
    const MAX_PARAMETERS: usize = 1024;
    const MAX_ENTITY_FIELDS: usize = 64;
    const MAX_IDENTIFIER_LENGTH: usize = 128;
    const MAX_ID_TEXT_LENGTH: usize = 256;
    const MAX_PAGE_POSITION: u64 = 1_000_000;
    const FORMAT_FAILURE: &str = "SQL statement formatting failed.";

    pub(super) fn parse(driver: &str) -> Result<Self, Failure> {
        Self::from_driver(driver).ok_or_else(|| failure("Unsupported Yon datasource driver."))
    }

    fn from_driver(driver: &str) -> Option<Self> {
        match driver {
            "postgresql" => Some(Self::Postgres),
            "mysql" | "mariadb" => Some(Self::Mysql),
            "sqlite" => Some(Self::Sqlite),
            "sqlserver" => Some(Self::SqlServer),
            _ => None,
        }
    }

    fn quote(self, name: &str) -> Result<String, Failure> {
        let name = Self::simple_name(name)?;
        Ok(self.quoted(name))
    }

    fn simple_name(name: &str) -> Result<&str, Failure> {
        if !is_simple_identifier(name) {
            return Err(failure("Storage identifiers require simple ASCII names."));
        }
        Ok(name)
    }

    fn quoted(self, name: &str) -> String {
        match self {
            Self::Mysql => format!("`{name}`"),
            Self::SqlServer => format!("[{name}]"),
            _ => format!("\"{name}\""),
        }
    }

    fn bind(self, parameters: &mut Vec<Value>, value: &Value) -> Result<String, Failure> {
        scalar(value)?;
        if value.is_null() {
            return Ok("NULL".into());
        }
        self.push_parameter(parameters, value)
    }

    fn push_parameter(self, parameters: &mut Vec<Value>, value: &Value) -> Result<String, Failure> {
        Self::require_parameter_room(parameters)?;
        parameters.push(value.clone());
        Ok(self.placeholder(parameters.len()))
    }

    fn require_parameter_room(parameters: &[Value]) -> Result<(), Failure> {
        if parameters.len() >= Self::MAX_PARAMETERS {
            return Err(failure("Storage parameter limit exceeded."));
        }
        Ok(())
    }

    fn placeholder(self, number: usize) -> String {
        match self {
            Self::Postgres => format!("${number}"),
            Self::SqlServer => format!("@P{number}"),
            _ => "?".into(),
        }
    }

    fn bindings<'value>(
        self,
        values: impl Iterator<Item = &'value Value>,
        parameters: &mut Vec<Value>,
    ) -> Result<Vec<String>, Failure> {
        let mut bindings = Vec::new();
        for value in values {
            bindings.push(self.bind(parameters, value)?);
        }
        Ok(bindings)
    }

    pub(super) fn control(self, action: &str) -> Statement {
        Statement {
            sql: self.control_sql(action).to_owned(),
            parameters: Vec::new(),
            result: ResultKind::Affected,
            isolated: false,
        }
    }

    fn control_sql(self, action: &str) -> &'static str {
        match (action, self) {
            ("begin", Self::SqlServer) => "BEGIN TRANSACTION",
            ("begin", _) => "BEGIN",
            ("commit", _) => "COMMIT",
            _ => "ROLLBACK",
        }
    }

    pub(super) fn compile(self, command: &Value) -> Result<Statement, Failure> {
        let operation = text(command, "operation")?;
        if operation == "sql" {
            return explicit_sql(command, self);
        }
        object_keys(command, &ManagedStorage::REPOSITORY_COMMAND_KEYS)?;
        let table = self.quote(text(command, "table")?)?;
        let id = self.quote(text(command, "idField")?)?;
        let mut statement = Statement {
            sql: String::new(),
            parameters: Vec::new(),
            result: ResultKind::Rows,
            isolated: false,
        };
        match operation {
            "save" => self.save(command, &table, &id, &mut statement)?,
            "delete" => self.delete(command, &table, &id, &mut statement)?,
            "find" => {
                statement.sql = format!("SELECT * FROM {table}");
                self.page(&command["page"], &id, &mut statement)?;
            }
            "query" => self.query(&command["query"], &table, &id, &mut statement)?,
            _ => return Err(failure("Unsupported managed storage operation.")),
        }
        Ok(statement)
    }

    fn delete(
        self,
        command: &Value,
        table: &str,
        id: &str,
        statement: &mut Statement,
    ) -> Result<(), Failure> {
        let value = command
            .get("id")
            .ok_or_else(|| failure("Store delete requires an identifier."))?;
        entity_id(value)?;
        let binding = self.bind(&mut statement.parameters, value)?;
        statement.sql = format!("DELETE FROM {table} WHERE {id} = {binding}");
        statement.result = ResultKind::Changed;
        Ok(())
    }

    fn save(
        self,
        command: &Value,
        table: &str,
        id: &str,
        statement: &mut Statement,
    ) -> Result<(), Failure> {
        let value = command
            .get("value")
            .ok_or_else(|| failure("Store save requires an entity."))?;
        let data = value
            .as_object()
            .filter(|data| (1..=Self::MAX_ENTITY_FIELDS).contains(&data.len()))
            .ok_or_else(|| failure("Store save requires a bounded entity object."))?;
        let identifier = data
            .get(text(command, "idField")?)
            .ok_or_else(|| failure("Store save requires its mapped identifier."))?;
        entity_id(identifier)?;
        let columns = self.quoted_columns(data.keys())?;
        let bindings = self.bindings(data.values(), &mut statement.parameters)?;
        let assignments = self.update_assignments(&columns, id);
        let upsert = Upsert {
            table,
            id,
            columns: &columns,
            bindings: &bindings,
            assignments: &assignments,
        };
        statement.sql = self.upsert_sql(&upsert);
        statement.result = ResultKind::Saved(value.clone());
        Ok(())
    }

    fn quoted_columns<'name>(
        self,
        names: impl Iterator<Item = &'name String>,
    ) -> Result<Vec<String>, Failure> {
        let mut columns = Vec::new();
        for name in names {
            columns.push(self.quote(name)?);
        }
        Ok(columns)
    }

    fn upsert_sql(self, upsert: &Upsert<'_>) -> String {
        if self == Self::SqlServer {
            return merge_sql(upsert);
        }
        self.insert_sql(upsert)
    }

    fn update_assignments(self, columns: &[String], id: &str) -> Vec<String> {
        columns
            .iter()
            .filter(|column| column.as_str() != id)
            .map(|column| self.assignment(column))
            .collect()
    }

    fn assignment(self, column: &str) -> String {
        match self {
            Self::Mysql => format!("{column} = VALUES({column})"),
            Self::SqlServer => format!("target.{column} = source.{column}"),
            _ => format!("{column} = excluded.{column}"),
        }
    }

    fn insert_sql(self, upsert: &Upsert<'_>) -> String {
        let Upsert {
            table,
            id,
            columns,
            bindings,
            assignments,
        } = upsert;
        let columns_joined = columns.join(", ");
        let bindings_joined = bindings.join(", ");
        let suffix = self.conflict_clause(id, assignments);
        format!("INSERT INTO {table} ({columns_joined}) VALUES ({bindings_joined}){suffix}")
    }

    fn conflict_clause(self, id: &str, assignments: &[String]) -> String {
        let updates = assignments.join(", ");
        if self == Self::Mysql && assignments.is_empty() {
            format!(" ON DUPLICATE KEY UPDATE {id} = {id}")
        } else if self == Self::Mysql {
            format!(" ON DUPLICATE KEY UPDATE {updates}")
        } else if assignments.is_empty() {
            format!(" ON CONFLICT ({id}) DO NOTHING")
        } else {
            format!(" ON CONFLICT ({id}) DO UPDATE SET {updates}")
        }
    }

    fn query(
        self,
        query: &Value,
        table: &str,
        id: &str,
        statement: &mut Statement,
    ) -> Result<(), Failure> {
        object_keys(query, &["kind", "where", "orderBy", "page"])?;
        let result = Self::query_result(text(query, "kind")?)?;
        statement.sql = self.select_sql(&result, table);
        self.where_clause(query, statement)?;
        self.finish_query(result, query, id, statement)
    }

    fn query_result(kind: &str) -> Result<ResultKind, Failure> {
        match kind {
            "find" => Ok(ResultKind::Rows),
            "count" => Ok(ResultKind::Count),
            "exists" => Ok(ResultKind::Exists),
            "delete" => Ok(ResultKind::Deleted),
            _ => Err(failure("Unsupported repository query kind.")),
        }
    }

    fn select_sql(self, result: &ResultKind, table: &str) -> String {
        match (result, self) {
            (ResultKind::Rows, _) => format!("SELECT * FROM {table}"),
            (ResultKind::Deleted, _) => format!("DELETE FROM {table}"),
            (_, Self::SqlServer) => format!("SELECT COUNT_BIG(*) AS tachyon_count FROM {table}"),
            _ => format!("SELECT COUNT(*) AS tachyon_count FROM {table}"),
        }
    }

    fn finish_query(
        self,
        result: ResultKind,
        query: &Value,
        id: &str,
        statement: &mut Statement,
    ) -> Result<(), Failure> {
        if let ResultKind::Rows = result {
            let page = page_with_query_order(query)?;
            return self.page(&page, id, statement);
        }
        reject_count_page_or_order(query)?;
        statement.result = result;
        Ok(())
    }

    fn where_clause(self, query: &Value, statement: &mut Statement) -> Result<(), Failure> {
        let predicates = query["where"]
            .as_array()
            .filter(|items| items.len() <= 8)
            .ok_or_else(|| failure("Store query requires bounded predicates."))?;
        let clauses = self.predicate_clauses(predicates, &mut statement.parameters)?;
        Self::append_where(statement, &clauses)
    }

    fn predicate_clauses(
        self,
        predicates: &[Value],
        parameters: &mut Vec<Value>,
    ) -> Result<Vec<String>, Failure> {
        let mut clauses = Vec::new();
        for predicate in predicates {
            clauses.push(self.predicate(predicate, parameters)?);
        }
        Ok(clauses)
    }

    fn append_where(statement: &mut Statement, clauses: &[String]) -> Result<(), Failure> {
        if clauses.is_empty() {
            return Ok(());
        }
        write!(statement.sql, " WHERE {}", clauses.join(" AND "))
            .map_err(|_| failure(Self::FORMAT_FAILURE))
    }

    fn predicate(self, predicate: &Value, parameters: &mut Vec<Value>) -> Result<String, Failure> {
        object_keys(predicate, &["field", "operator", "value"])?;
        let field = self.quote(text(predicate, "field")?)?;
        let operator = text(predicate, "operator")?;
        match operator {
            "isNull" => Ok(format!("{field} IS NULL")),
            "isNotNull" => Ok(format!("{field} IS NOT NULL")),
            "in" => self.in_predicate(&field, predicate, parameters),
            _ => self.comparison_predicate(&field, operator, predicate, parameters),
        }
    }

    fn in_predicate(
        self,
        field: &str,
        predicate: &Value,
        parameters: &mut Vec<Value>,
    ) -> Result<String, Failure> {
        let values = predicate["value"]
            .as_array()
            .filter(|values| values.len() <= 100)
            .ok_or_else(|| failure("In predicate requires a bounded array."))?;
        let bindings = self.bindings(values.iter(), parameters)?;
        Ok(Self::in_list(field, &bindings))
    }

    fn in_list(field: &str, bindings: &[String]) -> String {
        if bindings.is_empty() {
            return "1 = 0".into();
        }
        format!("{field} IN ({})", bindings.join(", "))
    }

    fn comparison_predicate(
        self,
        field: &str,
        operator: &str,
        predicate: &Value,
        parameters: &mut Vec<Value>,
    ) -> Result<String, Failure> {
        let comparison = match operator {
            "eq" => "=",
            "ne" => "<>",
            "lt" => "<",
            "lte" => "<=",
            "gt" => ">",
            "gte" => ">=",
            _ => return Err(failure("Unsupported repository comparison.")),
        };
        let value = predicate
            .get("value")
            .ok_or_else(|| failure("Query predicate requires a value."))?;
        if value.is_null() && operator == "eq" {
            return Ok(format!("{field} IS NULL"));
        }
        if value.is_null() && operator == "ne" {
            return Ok(format!("{field} IS NOT NULL"));
        }
        let binding = self.bind(parameters, value)?;
        Ok(format!("{field} {comparison} {binding}"))
    }

    fn page(self, page: &Value, id: &str, statement: &mut Statement) -> Result<(), Failure> {
        let page = Self::page_or_empty(page);
        object_keys(&page, &["page", "size", "sort", "offset"])?;
        let (index, size) = page_index_and_size(&page)?;
        let order = Self::order_or_id(self.sort_order(&page)?, id);
        write!(statement.sql, " ORDER BY {}", order.join(", "))
            .map_err(|_| failure(Self::FORMAT_FAILURE))?;
        let limit = self.limit_clause(index * size, size, &mut statement.parameters)?;
        statement.sql.push_str(&limit);
        Ok(())
    }

    fn page_or_empty(page: &Value) -> Value {
        if page.is_null() {
            return json!({});
        }
        page.clone()
    }

    fn order_or_id(order: Vec<String>, id: &str) -> Vec<String> {
        if order.is_empty() {
            return vec![format!("{id} ASC")];
        }
        order
    }

    fn limit_clause(
        self,
        offset: u64,
        size: u64,
        parameters: &mut Vec<Value>,
    ) -> Result<String, Failure> {
        if self == Self::SqlServer {
            let skip = self.push_parameter(parameters, &json!(offset))?;
            let take = self.push_parameter(parameters, &json!(size))?;
            return Ok(format!(" OFFSET {skip} ROWS FETCH NEXT {take} ROWS ONLY"));
        }
        let take = self.push_parameter(parameters, &json!(size))?;
        let skip = self.push_parameter(parameters, &json!(offset))?;
        Ok(format!(" LIMIT {take} OFFSET {skip}"))
    }

    fn sort_order(self, page: &Value) -> Result<Vec<String>, Failure> {
        let items = Self::sort_items(page)?;
        self.order_terms(items)
    }

    fn sort_items(page: &Value) -> Result<&[Value], Failure> {
        let Some(sort) = page.get("sort") else {
            return Ok(&[]);
        };
        sort.as_array()
            .filter(|sort| sort.len() <= 5)
            .map(Vec::as_slice)
            .ok_or_else(|| failure("Invalid repository sorting."))
    }

    fn order_terms(self, items: &[Value]) -> Result<Vec<String>, Failure> {
        let mut order = Vec::new();
        for item in items {
            let (field, direction) = sort_field_and_direction(item)?;
            let field = self.quote(field)?;
            let direction = Self::sort_direction(direction)?;
            order.push(format!("{field} {direction}"));
        }
        Ok(order)
    }

    fn sort_direction(direction: &str) -> Result<&'static str, Failure> {
        match direction {
            "asc" => Ok("ASC"),
            "desc" => Ok("DESC"),
            _ => Err(failure("Invalid repository sort direction.")),
        }
    }

    fn matched_update(assignments: &[String]) -> String {
        if assignments.is_empty() {
            return String::new();
        }
        format!(" WHEN MATCHED THEN UPDATE SET {}", assignments.join(", "))
    }

    fn is_simple_name(name: &str) -> bool {
        (1..=Self::MAX_IDENTIFIER_LENGTH).contains(&name.len())
            && !name.starts_with(|character: char| character.is_ascii_digit())
            && name.bytes().all(Self::is_identifier_byte)
    }

    fn is_identifier_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }

    fn checked_sql(sql: &str) -> Result<&str, Failure> {
        Self::reject_null_byte(sql)?;
        let sql = sql.trim().trim_end_matches(';');
        Self::require_one_statement(sql)?;
        Ok(sql)
    }

    fn reject_null_byte(sql: &str) -> Result<(), Failure> {
        if sql.contains('\0') {
            return Err(failure("SQL contains a null byte."));
        }
        Ok(())
    }

    fn require_one_statement(sql: &str) -> Result<(), Failure> {
        let verb = sql
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        let allowed_verbs = [
            "SELECT", "INSERT", "UPDATE", "DELETE", "WITH", "CREATE", "ALTER", "DROP", "MERGE",
        ];
        if sql.contains(';') || !allowed_verbs.contains(&verb.as_str()) {
            return Err(failure(
                "Explicit SQL requires one data or schema statement, not session or transaction control; bind literal values as parameters.",
            ));
        }
        Ok(())
    }

    fn explicit_parameters(command: &Value) -> Result<&Vec<Value>, Failure> {
        let parameters = command["parameters"]
            .as_array()
            .filter(|parameters| parameters.len() <= Self::MAX_PARAMETERS)
            .ok_or_else(|| failure("SQL requires a bounded parameter array."))?;
        for value in parameters {
            scalar(value)?;
        }
        Ok(parameters)
    }

    fn explicit_result(mode: &str) -> Result<ResultKind, Failure> {
        match mode {
            "rows" => Ok(ResultKind::Rows),
            "execute" => Ok(ResultKind::Affected),
            _ => Err(failure("SQL mode must be rows or execute.")),
        }
    }

    fn reject_paging(query: &Value) -> Result<(), Failure> {
        let has_order = query
            .get("orderBy")
            .and_then(Value::as_array)
            .is_some_and(|order| !order.is_empty());
        if query.get("page").is_some() || has_order {
            return Err(failure(
                "Count and exists queries cannot paginate or order.",
            ));
        }
        Ok(())
    }

    fn field_and_direction(item: &Value) -> Result<(&str, &str), Failure> {
        if let Some(sort) = item.as_str() {
            return sort
                .split_once(':')
                .ok_or_else(|| failure("Invalid repository sorting."));
        }
        object_keys(item, &["field", "direction"])?;
        Ok((text(item, "field")?, text(item, "direction")?))
    }
}

struct Upsert<'statement> {
    table: &'statement str,
    id: &'statement str,
    columns: &'statement [String],
    bindings: &'statement [String],
    assignments: &'statement [String],
}

fn merge_sql(upsert: &Upsert<'_>) -> String {
    let Upsert {
        table,
        id,
        columns,
        bindings,
        assignments,
    } = upsert;
    let columns_joined = columns.join(", ");
    let bindings_joined = bindings.join(", ");
    let updates = Dialect::matched_update(assignments);
    let source_values = columns
        .iter()
        .map(|column| format!("source.{column}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "MERGE {table} WITH (HOLDLOCK) AS target USING (VALUES ({bindings_joined})) AS source ({columns_joined}) ON target.{id} = source.{id}{updates} WHEN NOT MATCHED THEN INSERT ({columns_joined}) VALUES ({source_values});"
    )
}

pub(super) fn page_with_query_order(query: &Value) -> Result<Value, Failure> {
    let mut page = query.get("page").cloned().unwrap_or_else(|| json!({}));
    let Some(order) = query
        .get("orderBy")
        .and_then(Value::as_array)
        .filter(|order| !order.is_empty())
    else {
        return Ok(page);
    };
    let page_has_sort = page
        .get("sort")
        .and_then(Value::as_array)
        .is_some_and(|sort| !sort.is_empty());
    if page_has_sort {
        return Err(failure("Store ordering is ambiguous."));
    }
    page["sort"] = Value::Array(order.clone());
    Ok(page)
}

pub(super) fn reject_count_page_or_order(query: &Value) -> Result<(), Failure> {
    Dialect::reject_paging(query)
}

fn page_index_and_size(page: &Value) -> Result<(u64, u64), Failure> {
    let index = page
        .get("page")
        .map_or(Some(0), Value::as_u64)
        .filter(|index| *index <= Dialect::MAX_PAGE_POSITION)
        .ok_or_else(|| failure("Invalid repository page."))?;
    let size = page
        .get("size")
        .map_or(Some(20), Value::as_u64)
        .filter(|size| (1..=ManagedStorage::MAX_ROWS as u64).contains(size))
        .ok_or_else(|| failure("Invalid repository page size."))?;
    let offset = index * size;
    let offset_mismatch = page
        .get("offset")
        .is_some_and(|declared| declared.as_u64() != Some(offset));
    if offset > Dialect::MAX_PAGE_POSITION || offset_mismatch {
        return Err(failure("Invalid repository page offset."));
    }
    Ok((index, size))
}

pub(super) fn sort_field_and_direction(item: &Value) -> Result<(&str, &str), Failure> {
    Dialect::field_and_direction(item)
}

pub(super) fn is_simple_identifier(name: &str) -> bool {
    Dialect::is_simple_name(name)
}

fn explicit_sql(command: &Value, dialect: Dialect) -> Result<Statement, Failure> {
    object_keys(command, &["operation", "sql", "parameters", "mode"])?;
    let sql = Dialect::checked_sql(text(command, "sql")?)?;
    let parameters = Dialect::explicit_parameters(command)?;
    let result = Dialect::explicit_result(text(command, "mode")?)?;
    Ok(Statement {
        sql: sql.into(),
        parameters: parameters.clone(),
        result,
        isolated: dialect != Dialect::Sqlite,
    })
}

fn entity_id(value: &Value) -> Result<(), Failure> {
    let is_integer = value.as_i64().is_some();
    let is_bounded_text = value
        .as_str()
        .map(str::len)
        .is_some_and(|length| (1..=Dialect::MAX_ID_TEXT_LENGTH).contains(&length));
    if is_integer || is_bounded_text {
        return Ok(());
    }
    Err(failure(
        "Store identifiers require bounded strings or signed integers.",
    ))
}
