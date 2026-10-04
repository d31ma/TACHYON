mod dialect;
mod drivers;
mod fylo;
mod sqlite;

use std::time::Duration;

use serde_json::Value;
use tokio::sync::Semaphore;

use crate::Failure;
use crate::failure::diagnostic;
use drivers::{RedisStore, SqlStore};
use fylo::FyloStore;

pub(crate) struct ManagedStorage {
    sql: Option<SqlStore>,
    fylo: Option<FyloStore>,
    redis: Option<RedisStore>,
    admission: Semaphore,
}

impl std::fmt::Debug for ManagedStorage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManagedStorage")
            .field(
                "datasource_configured",
                &(self.sql.is_some() || self.fylo.is_some()),
            )
            .field("redis_configured", &self.redis.is_some())
            .finish_non_exhaustive()
    }
}

impl ManagedStorage {
    const MAX_BYTES: usize = 1024 * 1024;
    const MAX_ROWS: usize = 100;
    const OPERATION_TIMEOUT: Duration = Duration::from_secs(5);
    const SCROLL_TIMEOUT: Duration = Duration::from_secs(30);
    const MAX_TRANSACTIONS: usize = 4;
    const TRANSACTION_TIMEOUT: Duration = Duration::from_secs(10);
    const REPOSITORY_COMMAND_KEYS: [&str; 8] = [
        "operation",
        "entity",
        "table",
        "idField",
        "page",
        "query",
        "value",
        "id",
    ];

    pub(crate) async fn connect(configuration: &Value) -> Result<Self, Failure> {
        tokio::time::timeout(Duration::from_secs(10), async {
            let yon = &configuration["yon"];
            let (sql, fylo) = match yon.get("datasource") {
                Some(value) if value["driver"] == "fylo" => {
                    (None, Some(FyloStore::connect(value).await?))
                }
                Some(value) => (Some(SqlStore::connect(value).await?), None),
                None => (None, None),
            };
            let redis = match yon.get("redis") {
                Some(value) => Some(RedisStore::connect(value).await?),
                None => None,
            };
            Ok(Self {
                sql,
                fylo,
                redis,
                admission: Semaphore::new(16),
            })
        })
        .await
        .map_err(|_| failure("Managed storage startup deadline exceeded."))?
    }

    pub(crate) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        check_size(command)?;
        let _permit = self
            .admission
            .try_acquire()
            .map_err(|_| failure("Managed storage admission is closed or full."))?;
        tokio::time::timeout(Self::deadline(command), self.dispatch(command))
            .await
            .map_err(|_| {
                failure("Managed storage operation deadline exceeded; writes may have committed.")
            })?
    }

    fn deadline(command: &Value) -> Duration {
        let is_scroll = command["operation"] == "document" && command["command"] == "scroll";
        if is_scroll {
            return Self::SCROLL_TIMEOUT;
        }
        Self::OPERATION_TIMEOUT
    }

    async fn dispatch(&self, command: &Value) -> Result<Value, Failure> {
        if command["operation"] == "transaction" {
            return self.transaction(command).await;
        }
        if let Some(transaction) = command.get("transaction") {
            return self.execute_in_transaction(command, transaction).await;
        }
        let needs_fylo = matches!(command["operation"].as_str(), Some("document" | "queue"));
        if needs_fylo && self.fylo.is_none() {
            return Err(failure(
                "Document and queue commands need the FYLO datasource (driver fylo).",
            ));
        }
        let result = if command["operation"] == "redis" {
            self.redis_store()?.execute(command).await?
        } else if let Some(fylo) = &self.fylo {
            fylo.execute(command).await?
        } else {
            self.sql_store()?.execute(command).await?
        };
        check_size(&result)?;
        Ok(result)
    }

    async fn execute_in_transaction(
        &self,
        command: &Value,
        transaction: &Value,
    ) -> Result<Value, Failure> {
        let Some(id) = transaction.as_str().filter(|id| id.len() <= 128) else {
            return Err(failure("Transaction identifiers are bounded strings."));
        };
        let Some(mut inner) = command.as_object().cloned() else {
            return Err(failure("Storage settings and commands require objects."));
        };
        inner.remove("transaction");
        if inner.get("operation").and_then(Value::as_str) == Some("redis") {
            return Err(failure(
                "Redis commands cannot join a datasource transaction.",
            ));
        }
        let result = self
            .sql_store()?
            .execute_in(id, &Value::Object(inner))
            .await?;
        check_size(&result)?;
        Ok(result)
    }

    fn sql_store(&self) -> Result<&SqlStore, Failure> {
        self.sql
            .as_ref()
            .ok_or_else(|| failure("Yon datasource is not configured."))
    }

    fn redis_store(&self) -> Result<&RedisStore, Failure> {
        self.redis
            .as_ref()
            .ok_or_else(|| failure("Yon Redis is not configured."))
    }

    async fn transaction(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &["operation", "action", "transaction"])?;
        let action = text(command, "action")?;
        if !matches!(action, "begin" | "commit" | "rollback") {
            return Err(failure(
                "Transaction action must be begin, commit or rollback.",
            ));
        }
        if self.fylo.is_some() {
            return Err(failure(
                "The FYLO datasource has no transactions; write each document command on its own.",
            ));
        }
        let sql = self.sql_store()?;
        if action == "begin" {
            if command.get("transaction").is_some() {
                return Err(failure("Beginning a transaction takes no identifier."));
            }
            return sql.begin().await;
        }
        let Some(id) = command
            .get("transaction")
            .and_then(Value::as_str)
            .filter(|id| id.len() <= 128)
        else {
            return Err(failure(
                "Committing or rolling back requires its transaction.",
            ));
        };
        sql.settle(id, action).await
    }

    pub(crate) async fn close(&self) {
        self.admission.close();
        if let Some(sql) = &self.sql {
            sql.close().await;
        }
        if let Some(fylo) = &self.fylo {
            fylo.close().await;
        }
        if let Some(redis) = &self.redis {
            redis.close().await;
        }
    }
}

fn failure(message: &'static str) -> Failure {
    Failure::one(diagnostic(2160, message, None, None))
}

fn check_size(value: &Value) -> Result<(), Failure> {
    let encoded = serde_json::to_vec(value).map_err(|_| failure("Invalid storage JSON."))?;
    if encoded.len() > ManagedStorage::MAX_BYTES {
        return Err(failure("Storage JSON exceeds its byte limit."));
    }
    Ok(())
}

fn scalar(value: &Value) -> Result<(), Failure> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
        Value::Number(number) if number.is_i64() || number.is_f64() => Ok(()),
        _ => Err(failure(
            "Storage parameters require signed integers, finite numbers, strings, booleans, or null.",
        )),
    }
}

fn object_keys(value: &Value, allowed: &[&str]) -> Result<(), Failure> {
    let object = value
        .as_object()
        .ok_or_else(|| failure("Storage settings and commands require objects."))?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(failure(
            "Storage settings or command contains an unsupported key.",
        ));
    }
    Ok(())
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, Failure> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty() && text.len() <= 65536)
        .ok_or_else(|| failure("Storage command requires a bounded nonempty string."))
}

#[cfg(test)]
mod tests;
