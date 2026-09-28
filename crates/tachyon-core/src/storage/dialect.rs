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
}

impl ResultKind {
    pub(super) fn returns_rows(&self) -> bool {
        matches!(self, Self::Rows | Self::Count | Self::Exists)
    }
}

impl Dialect {
    pub(super) fn parse(driver: &str) -> Result<Self, Failure> {
        match driver {
            "postgresql" => Ok(Self::Postgres),
            "mysql" | "mariadb" => Ok(Self::Mysql),
            "sqlite" => Ok(Self::Sqlite),
            "sqlserver" => Ok(Self::SqlServer),
            _ => Err(failure("Unsupported Yon datasource driver.")),
        }
    }

    fn quote(self, name: &str) -> Result<String, Failure> {
        if !is_simple_identifier(name) {
            return Err(failure("Storage identifiers require simple ASCII names."));
        }
        Ok(match self {
            Self::Mysql => format!("`{name}`"),
            Self::SqlServer => format!("[{name}]"),
            _ => format!("\"{name}\""),
        })
    }

    fn bind(self, parameters: &mut Vec<Value>, value: &Value) -> Result<String, Failure> {
        scalar(value)?;
        if value.is_null() {
            return Ok("NULL".into());
        }
        if parameters.len() >= 1024 {
            return Err(failure("Storage parameter limit exceeded."));
        }
        parameters.push(value.clone());
        Ok(match self {
            Self::Postgres => format!("${}", parameters.len()),
            Self::SqlServer => format!("@P{}", parameters.len()),
            _ => "?".into(),
        })
    }

    pub(super) fn control(self, action: &str) -> Statement {
        let sql = match (action, self) {
            ("begin", Self::SqlServer) => "BEGIN TRANSACTION",
            ("begin", _) => "BEGIN",
            ("commit", _) => "COMMIT",
            _ => "ROLLBACK",
        };
        Statement {
            sql: sql.to_owned(),
            parameters: Vec::new(),
            result: ResultKind::Affected,
            isolated: false,
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
            .filter(|data| !data.is_empty() && data.len() <= 64)
            .ok_or_else(|| failure("Store save requires a bounded entity object."))?;
        if !data.contains_key(text(command, "idField")?) {
            return Err(failure("Store save requires its mapped identifier."));
        }
        entity_id(&data[text(command, "idField")?])?;
        let mut columns = Vec::new();
        for name in data.keys() {
            columns.push(self.quote(name)?);
        }
        let mut bindings = Vec::new();
        for value in data.values() {
            bindings.push(self.bind(&mut statement.parameters, value)?);
        }
        let assignments = self.update_assignments(&columns, id);
        let upsert = Upsert {
            table,
            id,
            columns: &columns,
            bindings: &bindings,
            assignments: &assignments,
        };
        statement.sql = if self == Self::SqlServer {
            merge_sql(&upsert)
        } else {
            self.insert_sql(&upsert)
        };
        statement.result = ResultKind::Saved(value.clone());
        Ok(())
    }

    fn update_assignments(self, columns: &[String], id: &str) -> Vec<String> {
        let mut assignments = Vec::new();
        for column in columns {
            if column.as_str() == id {
                continue;
            }
            let assignment = match self {
                Self::Mysql => format!("{column} = VALUES({column})"),
                Self::SqlServer => format!("target.{column} = source.{column}"),
                _ => format!("{column} = excluded.{column}"),
            };
            assignments.push(assignment);
        }
        assignments
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
        let suffix = if self == Self::Mysql {
            let updates = if assignments.is_empty() {
                format!("{id} = {id}")
            } else {
                assignments.join(", ")
            };
            format!(" ON DUPLICATE KEY UPDATE {updates}")
        } else if assignments.is_empty() {
            format!(" ON CONFLICT ({id}) DO NOTHING")
        } else {
            format!(
                " ON CONFLICT ({id}) DO UPDATE SET {}",
                assignments.join(", ")
            )
        };
        format!("INSERT INTO {table} ({columns_joined}) VALUES ({bindings_joined}){suffix}")
    }

    fn query(
        self,
        query: &Value,
        table: &str,
        id: &str,
        statement: &mut Statement,
    ) -> Result<(), Failure> {
        object_keys(query, &["kind", "where", "orderBy", "page"])?;
        let kind = text(query, "kind")?;
        statement.sql = match kind {
            "find" => format!("SELECT * FROM {table}"),
            "count" | "exists" => {
                let count_function = self.count_function();
                format!("SELECT {count_function}(*) AS tachyon_count FROM {table}")
            }
            _ => return Err(failure("Unsupported repository query kind.")),
        };
        self.where_clause(query, statement)?;
        if kind == "find" {
            let page = page_with_query_order(query)?;
            return self.page(&page, id, statement);
        }
        reject_count_page_or_order(query)?;
        statement.result = if kind == "count" {
            ResultKind::Count
        } else {
            ResultKind::Exists
        };
        Ok(())
    }

    fn count_function(self) -> &'static str {
        if self == Self::SqlServer {
            return "COUNT_BIG";
        }
        "COUNT"
    }

    fn where_clause(self, query: &Value, statement: &mut Statement) -> Result<(), Failure> {
        let predicates = query["where"]
            .as_array()
            .filter(|items| items.len() <= 8)
            .ok_or_else(|| failure("Store query requires bounded predicates."))?;
        let mut clauses = Vec::new();
        for predicate in predicates {
            clauses.push(self.predicate(predicate, &mut statement.parameters)?);
        }
        if clauses.is_empty() {
            return Ok(());
        }
        write!(statement.sql, " WHERE {}", clauses.join(" AND "))
            .map_err(|_| failure("SQL statement formatting failed."))
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
        if values.is_empty() {
            return Ok("1 = 0".into());
        }
        let mut bindings = Vec::new();
        for value in values {
            bindings.push(self.bind(parameters, value)?);
        }
        Ok(format!("{field} IN ({})", bindings.join(", ")))
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
        let page = if page.is_null() {
            json!({})
        } else {
            page.clone()
        };
        object_keys(&page, &["page", "size", "sort", "offset"])?;
        let (index, size) = page_index_and_size(&page)?;
        let mut order = self.sort_order(&page)?;
        if order.is_empty() {
            order.push(format!("{id} ASC"));
        }
        write!(statement.sql, " ORDER BY {}", order.join(", "))
            .map_err(|_| failure("SQL statement formatting failed."))?;
        let offset = json!(index * size);
        if self == Self::SqlServer {
            let skip = self.bind(&mut statement.parameters, &offset)?;
            let take = self.bind(&mut statement.parameters, &json!(size))?;
            write!(
                statement.sql,
                " OFFSET {skip} ROWS FETCH NEXT {take} ROWS ONLY"
            )
            .map_err(|_| failure("SQL statement formatting failed."))?;
        } else {
            let take = self.bind(&mut statement.parameters, &json!(size))?;
            let skip = self.bind(&mut statement.parameters, &offset)?;
            write!(statement.sql, " LIMIT {take} OFFSET {skip}")
                .map_err(|_| failure("SQL statement formatting failed."))?;
        }
        Ok(())
    }

    fn sort_order(self, page: &Value) -> Result<Vec<String>, Failure> {
        let mut order = Vec::new();
        let Some(sort) = page.get("sort") else {
            return Ok(order);
        };
        let sort = sort
            .as_array()
            .filter(|sort| sort.len() <= 5)
            .ok_or_else(|| failure("Invalid repository sorting."))?;
        for item in sort {
            let (field, direction) = sort_field_and_direction(item)?;
            let field = self.quote(field)?;
            let direction = match direction {
                "asc" => "ASC",
                "desc" => "DESC",
                _ => return Err(failure("Invalid repository sort direction.")),
            };
            order.push(format!("{field} {direction}"));
        }
        Ok(order)
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
    let updates = if assignments.is_empty() {
        String::new()
    } else {
        format!(" WHEN MATCHED THEN UPDATE SET {}", assignments.join(", "))
    };
    let mut source_columns = Vec::new();
    for column in *columns {
        source_columns.push(format!("source.{column}"));
    }
    let source_values = source_columns.join(", ");
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

fn page_index_and_size(page: &Value) -> Result<(u64, u64), Failure> {
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
    let offset = index * size;
    let offset_mismatch = page
        .get("offset")
        .is_some_and(|declared| declared.as_u64() != Some(offset));
    if offset > 1_000_000 || offset_mismatch {
        return Err(failure("Invalid repository page offset."));
    }
    Ok((index, size))
}

pub(super) fn sort_field_and_direction(item: &Value) -> Result<(&str, &str), Failure> {
    if let Some(sort) = item.as_str() {
        return sort
            .split_once(':')
            .ok_or_else(|| failure("Invalid repository sorting."));
    }
    object_keys(item, &["field", "direction"])?;
    Ok((text(item, "field")?, text(item, "direction")?))
}

pub(super) fn is_simple_identifier(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 {
        return false;
    }
    for (index, byte) in name.bytes().enumerate() {
        let allowed =
            byte.is_ascii_alphabetic() || byte == b'_' || (index > 0 && byte.is_ascii_digit());
        if !allowed {
            return false;
        }
    }
    true
}

fn explicit_sql(command: &Value, dialect: Dialect) -> Result<Statement, Failure> {
    object_keys(command, &["operation", "sql", "parameters", "mode"])?;
    let sql = text(command, "sql")?;
    if sql.contains('\0') {
        return Err(failure("SQL contains a null byte."));
    }
    let sql = sql.trim().trim_end_matches(';');
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
    let parameters = command["parameters"]
        .as_array()
        .filter(|parameters| parameters.len() <= 1024)
        .ok_or_else(|| failure("SQL requires a bounded parameter array."))?;
    for value in parameters {
        scalar(value)?;
    }
    let result = match text(command, "mode")? {
        "rows" => ResultKind::Rows,
        "execute" => ResultKind::Affected,
        _ => return Err(failure("SQL mode must be rows or execute.")),
    };
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
        .is_some_and(|text| !text.is_empty() && text.len() <= 256);
    if is_integer || is_bounded_text {
        return Ok(());
    }
    Err(failure(
        "Store identifiers require bounded strings or signed integers.",
    ))
}
