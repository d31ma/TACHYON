use std::str::FromStr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

use futures_util::TryStreamExt;
use serde_json::{Map, Value, json};
use sqlx::{Column, ConnectOptions, Row, TypeInfo, ValueRef};

use super::dialect::Statement;
use super::drivers::{finite_number, mutation_result, push_row, rows_result};
use super::{ManagedStorage, failure};
use crate::Failure;

pub(super) struct SqliteStore {
    pool: sqlx::SqlitePool,
}

struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

impl SqliteStore {
    pub(super) async fn connect(url: &str) -> Result<Self, Failure> {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(url)
            .map_err(|_| failure("Invalid SQLite URL."))?
            .create_if_missing(false)
            .busy_timeout(ManagedStorage::OPERATION_TIMEOUT)
            .disable_statement_logging();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(if url == "sqlite::memory:" { 1 } else { 8 })
            .test_before_acquire(false)
            .idle_timeout(None)
            .max_lifetime(None)
            .acquire_timeout(ManagedStorage::OPERATION_TIMEOUT)
            .connect_with(options)
            .await
            .map_err(|_| {
                failure("SQLite connection failed; the database file must already exist.")
            })?;
        Ok(Self { pool })
    }

    pub(super) async fn acquire_connection(
        &self,
    ) -> Result<sqlx::pool::PoolConnection<sqlx::Sqlite>, Failure> {
        self.pool
            .acquire()
            .await
            .map_err(|_| failure("SQLite admission failed."))
    }

    pub(super) async fn execute(&self, statement: &Statement) -> Result<Value, Failure> {
        let mut connection = self.acquire_connection().await?;
        Self::execute_on(&mut connection, statement).await
    }

    pub(super) async fn execute_on(
        connection: &mut sqlx::SqliteConnection,
        statement: &Statement,
    ) -> Result<Value, Failure> {
        let cancellation = CancelOnDrop(Arc::new(AtomicBool::new(false)));
        let cancelled = Arc::clone(&cancellation.0);
        let deadline = Instant::now() + ManagedStorage::OPERATION_TIMEOUT;
        connection
            .lock_handle()
            .await
            .map_err(|_| failure("SQLite worker access failed."))?
            .set_progress_handler(1000, move || {
                !cancelled.load(Ordering::Relaxed) && Instant::now() < deadline
            });
        let result = run_statement(connection, statement).await;
        connection
            .lock_handle()
            .await
            .map_err(|_| failure("SQLite worker settlement failed."))?
            .remove_progress_handler();
        result
    }

    pub(super) async fn close(&self) {
        self.pool.close().await;
    }
}

async fn run_statement(
    connection: &mut sqlx::SqliteConnection,
    statement: &Statement,
) -> Result<Value, Failure> {
    let mut query = sqlx::query(sqlx::AssertSqlSafe(statement.sql.clone()));
    for parameter in &statement.parameters {
        query = match parameter {
            Value::Null => query.bind(Option::<String>::None),
            Value::Bool(value) => query.bind(*value),
            Value::Number(value) if value.is_i64() => query.bind(value.as_i64()),
            Value::Number(value) => query.bind(value.as_f64()),
            Value::String(value) => query.bind(value.clone()),
            _ => return Err(failure("Invalid SQLite parameter.")),
        };
    }
    if !statement.result.returns_rows() {
        let affected = query
            .execute(connection)
            .await
            .map_err(|_| {
                failure("SQLite mutation failed or was interrupted; no retry was performed.")
            })?
            .rows_affected();
        return Ok(mutation_result(&statement.result, affected));
    }
    let mut stream = query.fetch(connection);
    let mut rows = Vec::new();
    let mut bytes = 0;
    while let Some(row) = stream
        .try_next()
        .await
        .map_err(|_| failure("SQLite query failed or was interrupted."))?
    {
        push_row(&mut rows, row_json(&row)?, &mut bytes)?;
    }
    rows_result(&statement.result, rows)
}

fn row_json(row: &sqlx::sqlite::SqliteRow) -> Result<Value, Failure> {
    if row.columns().len() > 64 {
        return Err(failure("Storage row column limit exceeded."));
    }
    let mut object = Map::new();
    for (index, column) in row.columns().iter().enumerate() {
        if object.contains_key(column.name()) {
            return Err(failure("SQL result column names must be unique."));
        }
        let value = column_json(row, index)?;
        object.insert(column.name().into(), value);
    }
    Ok(Value::Object(object))
}

fn column_json(row: &sqlx::sqlite::SqliteRow, index: usize) -> Result<Value, Failure> {
    let raw = row
        .try_get_raw(index)
        .map_err(|_| failure("SQLite row decode failed."))?;
    if raw.is_null() {
        return Ok(Value::Null);
    }
    match raw.type_info().name() {
        "INTEGER" => {
            let value = row
                .try_get::<i64, _>(index)
                .map_err(|_| failure("SQLite integer decode failed."))?;
            Ok(json!(value))
        }
        "REAL" => {
            let value = row
                .try_get::<f64, _>(index)
                .map_err(|_| failure("SQLite number decode failed."))?;
            finite_number(value)
        }
        "TEXT" => {
            let value = row
                .try_get::<String, _>(index)
                .map_err(|_| failure("SQLite text decode failed."))?;
            Ok(json!(value))
        }
        "BLOB" => {
            let value = row
                .try_get::<Vec<u8>, _>(index)
                .map_err(|_| failure("SQLite binary decode failed."))?;
            Ok(json!(value))
        }
        _ => Err(failure("SQLite column is not JSON-compatible.")),
    }
}
