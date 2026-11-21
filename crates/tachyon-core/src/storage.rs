mod busy_clock;
mod dialect;
mod drivers;
mod fylo;
#[cfg(feature = "storage-sqlserver")]
mod prelogin_stream;
#[cfg(feature = "storage-redis")]
mod redis_store;
mod sqlite;
mod startup_activity;
mod startup_allowance;
mod store_call;
mod store_call_log;
mod store_operation;
#[cfg(not(feature = "storage-redis"))]
mod unbuilt_redis_store;

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::Semaphore;

use crate::Failure;
use crate::failure::diagnostic;
pub(crate) use busy_clock::BusyClock;
use drivers::SqlStore;
use fylo::FyloStore;
#[cfg(feature = "storage-redis")]
use redis_store::RedisStore;
use startup_activity::StartupActivity;
use startup_allowance::StartupAllowance;
pub(crate) use store_call::StoreCall;
pub(crate) use store_call_log::StoreCallLog;
pub(crate) use store_operation::StoreOperation;
#[cfg(not(feature = "storage-redis"))]
use unbuilt_redis_store::UnbuiltRedisStore as RedisStore;

pub(crate) struct ManagedStorage {
    sql: Option<SqlStore>,
    fylo: Option<FyloStore>,
    redis: Option<RedisStore>,
    admission: Semaphore,
    deadline_factor: u32,
    call_log: StoreCallLog,
    startup_allowance: Duration,
    startup_activity: Arc<StartupActivity>,
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
    const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
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
        Self::connect_with_deadline_factor(configuration, 1).await
    }

    async fn connect_with_deadline_factor(
        configuration: &Value,
        deadline_factor: u32,
    ) -> Result<Self, Failure> {
        tokio::time::timeout(Self::STARTUP_TIMEOUT * deadline_factor, async {
            let yon = &configuration["yon"];
            let call_log = StoreCallLog::from_configuration(yon)?;
            let startup_allowance = StartupAllowance::from_configuration(yon)?;
            let datasource = StartupAllowance::without_its_setting(
                StoreCallLog::without_its_settings(yon.get("datasource").cloned()),
            );
            let (sql, fylo) = match &datasource {
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
                deadline_factor,
                call_log,
                startup_allowance: startup_allowance * deadline_factor,
                startup_activity: Arc::new(StartupActivity::new()),
            })
        })
        .await
        .map_err(|_| failure("Managed storage startup deadline exceeded."))?
    }

    #[cfg(test)]
    pub(crate) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        self.execute_within(command, None).await
    }

    #[cfg(test)]
    pub(crate) async fn execute_within(
        &self,
        command: &Value,
        shared_deadline: Option<Instant>,
    ) -> Result<Value, Failure> {
        let allowance = self.startup_allowance();
        let (result, _call) = self
            .execute_measured(command, shared_deadline, allowance)
            .await;
        result
    }

    #[cfg(test)]
    pub(crate) async fn execute_measured(
        &self,
        command: &Value,
        shared_deadline: Option<Instant>,
        startup_left: Duration,
    ) -> (Result<Value, Failure>, Arc<StoreCall>) {
        let (call, deadline) = self.prepare_call(command, shared_deadline, startup_left);
        let result = self.execute_prepared(command, deadline, &call).await;
        (result, call)
    }

    pub(crate) fn prepare_call(
        &self,
        command: &Value,
        shared_deadline: Option<Instant>,
        startup_left: Duration,
    ) -> (Arc<StoreCall>, Instant) {
        let deadline = Self::deadline_of(command, shared_deadline, self.time_limit(command));
        let activity = Arc::clone(&self.startup_activity);
        (StoreCall::new(deadline, startup_left, activity), deadline)
    }

    pub(crate) fn startup_allowance(&self) -> Duration {
        self.startup_allowance
    }

    pub(crate) fn call_log(&self) -> &StoreCallLog {
        &self.call_log
    }

    pub(crate) async fn execute_prepared(
        &self,
        command: &Value,
        deadline: Instant,
        call: &Arc<StoreCall>,
    ) -> Result<Value, Failure> {
        check_size(command)?;
        let _permit = self
            .admission
            .try_acquire()
            .map_err(|_| failure("Managed storage admission is closed or full."))?;
        self.refuse_when_past(deadline)?;
        call.run(self.dispatch(command, deadline))
            .await
            .unwrap_or_else(|| Err(self.deadline_failure()))
    }

    pub(crate) fn operation_timeout(&self) -> Duration {
        let base = self
            .fylo
            .as_ref()
            .map_or(Self::OPERATION_TIMEOUT, FyloStore::operation_timeout);
        base * self.deadline_factor
    }

    fn deadline_of(command: &Value, shared: Option<Instant>, time_limit: Duration) -> Instant {
        let own = Instant::now() + time_limit;
        if Self::is_scroll(command) {
            return own;
        }
        shared.unwrap_or(own)
    }

    fn refuse_when_past(&self, deadline: Instant) -> Result<(), Failure> {
        if Instant::now() >= deadline {
            return Err(self.deadline_failure());
        }
        Ok(())
    }

    fn is_scroll(command: &Value) -> bool {
        let names = (command["operation"].as_str(), command["command"].as_str());
        names == (Some("document"), Some("scroll"))
    }

    fn deadline_failure(&self) -> Failure {
        let fylo_failure = self.fylo.as_ref().and_then(FyloStore::deadline_failure);
        fylo_failure.unwrap_or_else(|| {
            failure("Managed storage operation deadline exceeded; the store-time budget is spent and writes may have committed.")
        })
    }

    fn time_limit(&self, command: &Value) -> Duration {
        if Self::is_scroll(command) {
            return Self::SCROLL_TIMEOUT * self.deadline_factor;
        }
        self.operation_timeout()
    }

    async fn dispatch(&self, command: &Value, deadline: Instant) -> Result<Value, Failure> {
        if command["operation"] == "transaction" {
            return self.transaction(command, deadline).await;
        }
        if let Some(transaction) = command.get("transaction") {
            return self
                .execute_in_transaction(command, transaction, deadline)
                .await;
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
            self.sql_store()?.execute(command, deadline).await?
        };
        check_size(&result)?;
        Ok(result)
    }

    async fn execute_in_transaction(
        &self,
        command: &Value,
        transaction: &Value,
        deadline: Instant,
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
            .execute_in(id, &Value::Object(inner), deadline)
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

    async fn transaction(&self, command: &Value, deadline: Instant) -> Result<Value, Failure> {
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
            return sql.begin(deadline).await;
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
        sql.settle(id, action, deadline).await
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
