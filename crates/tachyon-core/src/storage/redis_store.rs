use redis::aio::ConnectionManager;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use super::drivers::connection_url;
use super::{ManagedStorage, failure, object_keys, text};
use crate::Failure;

pub(super) struct RedisStore {
    connection: Mutex<Option<ConnectionManager>>,
}

impl RedisStore {
    pub(super) async fn connect(settings: &Value) -> Result<Self, Failure> {
        object_keys(settings, &["url", "username", "password"])?;
        let raw = text(settings, "url")?;
        let scheme = if raw.starts_with("rediss:") {
            "rediss"
        } else {
            "redis"
        };
        let url = connection_url(settings, scheme)?;
        let client = redis::Client::open(url.as_str())
            .map_err(|_| failure("Invalid Redis connection configuration."))?;
        let connection = ConnectionManager::new(client)
            .await
            .map_err(|_| failure("Redis connection failed."))?;
        Ok(Self {
            connection: Mutex::new(Some(connection)),
        })
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(
            command,
            &["operation", "command", "key", "value", "ttlSeconds"],
        )?;
        let key = text(command, "key")?;
        if key.len() > 4096 {
            return Err(failure("Redis key is too large."));
        }
        let mut connection = self
            .connection
            .lock()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| failure("Redis resource is closed."))?;
        match text(command, "command")? {
            "get" => redis_get(&mut connection, key).await,
            "set" => redis_set(&mut connection, key, command).await,
            "delete" => redis_delete(&mut connection, key).await,
            _ => Err(failure("Unsupported Redis operation.")),
        }
    }

    pub(super) async fn close(&self) {
        self.connection.lock().await.take();
    }
}

async fn redis_get(connection: &mut ConnectionManager, key: &str) -> Result<Value, Failure> {
    let value: Vec<u8> = redis::cmd("GETRANGE")
        .arg(key)
        .arg(0)
        .arg(ManagedStorage::MAX_BYTES)
        .query_async(connection)
        .await
        .map_err(|_| failure("Redis read failed."))?;
    if value.is_empty() {
        return Ok(Value::Null);
    }
    if value.len() > ManagedStorage::MAX_BYTES {
        return Err(failure("Redis value exceeds byte limit."));
    }
    serde_json::from_slice(&value).map_err(|_| failure("Redis value is not valid JSON."))
}

async fn redis_set(
    connection: &mut ConnectionManager,
    key: &str,
    command: &Value,
) -> Result<Value, Failure> {
    let value = command
        .get("value")
        .ok_or_else(|| failure("Redis set requires a JSON value."))?;
    let bytes = serde_json::to_vec(value).map_err(|_| failure("Redis JSON encoding failed."))?;
    let ttl = command["ttlSeconds"]
        .as_u64()
        .filter(|ttl| *ttl > 0 && *ttl <= 86400)
        .ok_or_else(|| failure("Redis set requires a TTL between 1 and 86400 seconds."))?;
    redis::cmd("SET")
        .arg(key)
        .arg(bytes)
        .arg("EX")
        .arg(ttl)
        .query_async::<()>(connection)
        .await
        .map_err(|_| failure("Redis write failed; no retry was performed."))?;
    Ok(Value::Bool(true))
}

async fn redis_delete(connection: &mut ConnectionManager, key: &str) -> Result<Value, Failure> {
    let affected: u64 = redis::cmd("DEL")
        .arg(key)
        .query_async(connection)
        .await
        .map_err(|_| failure("Redis delete failed."))?;
    Ok(json!(affected > 0))
}
