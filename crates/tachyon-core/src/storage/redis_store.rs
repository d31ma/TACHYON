mod redis_channel;
mod redis_reply;
mod redis_target;
#[cfg(test)]
mod tests;

use serde_json::{Value, json};
use tokio::sync::Mutex;

use super::{ManagedStorage, failure, object_keys, text};
use crate::Failure;
use redis_channel::RedisChannel;
use redis_reply::RedisReply;
use redis_target::RedisTarget;

/// Tachyon's own Redis client: the three commands Yon offers (`GETRANGE`, `SET … EX`, `DEL`)
/// over one multiplexed RESP2 connection that is reopened, with backoff, after it drops.
/// A command that was in flight when the connection dropped fails; it is never retried.
pub(super) struct RedisStore {
    target: RedisTarget,
    channel: Mutex<Option<RedisChannel>>,
}

impl RedisStore {
    const MAX_KEY_BYTES: usize = 4096;
    const MAX_TTL_SECONDS: u64 = 86400;

    pub(super) async fn connect(settings: &Value) -> Result<Self, Failure> {
        object_keys(settings, &["url", "username", "password"])?;
        let target = RedisTarget::from_settings(settings)?;
        let channel = target
            .connect()
            .await
            .map_err(|_| failure("Redis connection failed."))?;
        Ok(Self {
            target,
            channel: Mutex::new(Some(channel)),
        })
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(
            command,
            &["operation", "command", "key", "value", "ttlSeconds"],
        )?;
        let key = Self::key(command)?;
        let channel = self.channel().await?;
        Self::dispatch(&channel, text(command, "command")?, key, command).await
    }

    pub(super) async fn close(&self) {
        self.channel.lock().await.take();
    }

    fn key(command: &Value) -> Result<&str, Failure> {
        let key = text(command, "key")?;
        if key.len() > Self::MAX_KEY_BYTES {
            return Err(failure("Redis key is too large."));
        }
        Ok(key)
    }

    async fn channel(&self) -> Result<RedisChannel, Failure> {
        let mut slot = self.channel.lock().await;
        let channel = slot
            .as_mut()
            .ok_or_else(|| failure("Redis resource is closed."))?;
        self.reopen(channel).await;
        Ok(channel.clone())
    }

    /// Replaces a dropped connection. If Redis is still unreachable the dropped one stays, so the
    /// command fails with its own message and the next command tries again.
    async fn reopen(&self, channel: &mut RedisChannel) {
        if channel.is_closed() {
            let dropped = channel.clone();
            *channel = self.target.connect().await.unwrap_or(dropped);
        }
    }

    async fn dispatch(
        channel: &RedisChannel,
        name: &str,
        key: &str,
        command: &Value,
    ) -> Result<Value, Failure> {
        match name {
            "get" => Self::get(channel, key).await,
            "set" => Self::set(channel, key, command).await,
            "delete" => Self::delete(channel, key).await,
            _ => Err(failure("Unsupported Redis operation.")),
        }
    }

    async fn get(channel: &RedisChannel, key: &str) -> Result<Value, Failure> {
        let last = ManagedStorage::MAX_BYTES.to_string();
        let value = channel
            .call(&[b"GETRANGE", key.as_bytes(), b"0", last.as_bytes()])
            .await
            .and_then(RedisReply::into_bytes)
            .ok_or_else(|| failure("Redis read failed."))?;
        Self::decode(&value)
    }

    fn decode(value: &[u8]) -> Result<Value, Failure> {
        match (value.is_empty(), value.len() > ManagedStorage::MAX_BYTES) {
            (true, _) => Ok(Value::Null),
            (false, true) => Err(failure("Redis value exceeds byte limit.")),
            (false, false) => {
                serde_json::from_slice(value).map_err(|_| failure("Redis value is not valid JSON."))
            }
        }
    }

    async fn set(channel: &RedisChannel, key: &str, command: &Value) -> Result<Value, Failure> {
        let value = command
            .get("value")
            .ok_or_else(|| failure("Redis set requires a JSON value."))?;
        let bytes =
            serde_json::to_vec(value).map_err(|_| failure("Redis JSON encoding failed."))?;
        let ttl = command["ttlSeconds"]
            .as_u64()
            .filter(|ttl| (1..=Self::MAX_TTL_SECONDS).contains(ttl))
            .ok_or_else(|| failure("Redis set requires a TTL between 1 and 86400 seconds."))?
            .to_string();
        channel
            .call(&[b"SET", key.as_bytes(), &bytes, b"EX", ttl.as_bytes()])
            .await
            .ok_or_else(|| failure("Redis write failed; no retry was performed."))?;
        Ok(Value::Bool(true))
    }

    async fn delete(channel: &RedisChannel, key: &str) -> Result<Value, Failure> {
        let affected = channel
            .call(&[b"DEL", key.as_bytes()])
            .await
            .and_then(RedisReply::into_integer)
            .ok_or_else(|| failure("Redis delete failed."))?;
        Ok(json!(affected > 0))
    }
}
