use std::ops::RangeInclusive;

use serde_json::{Value, json};

use super::queue_name::QueueName;
use super::{FyloError, FyloStore};
use crate::Failure;
use crate::storage::{failure, object_keys, text};

pub(super) struct QueueCommands<'store> {
    store: &'store FyloStore,
}

impl<'store> QueueCommands<'store> {
    const PUBLISH_KEYS: [&'static str; 6] =
        ["operation", "command", "topic", "payload", "key", "delayMs"];
    const CLAIM_KEYS: [&'static str; 7] = [
        "operation",
        "command",
        "topic",
        "group",
        "maxMessages",
        "leaseMs",
        "maxAttempts",
    ];
    const SETTLE_KEYS: [&'static str; 7] = [
        "operation",
        "command",
        "topic",
        "group",
        "id",
        "receipt",
        "delayMs",
    ];
    const DELIVERY_FIELDS: [&'static str; 4] = ["id", "receipt", "attempt", "payload"];
    const MAX_KEY_LENGTH: usize = 256;
    const MAX_TOKEN_LENGTH: usize = 128;
    const MAX_DELAY_MS: u64 = 30 * 24 * 60 * 60 * 1000;
    const MAX_LEASE_MS: u64 = 24 * 60 * 60 * 1000;
    const DEFAULT_LEASE_MS: u64 = 30_000;
    const MAX_MESSAGES: u64 = 100;
    const MAX_ATTEMPTS: u64 = 100;
    const DEFAULT_ATTEMPTS: u64 = 3;

    pub(super) fn new(store: &'store FyloStore) -> Self {
        Self { store }
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        match text(command, "command")? {
            "publish" => self.publish(command).await,
            "claim" => self.claim(command).await,
            "ack" => self.settle(command, "queueAck").await,
            "nack" => self.settle(command, "queueNack").await,
            _ => Err(failure("Unsupported FYLO queue command.")),
        }
    }

    async fn publish(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::PUBLISH_KEYS)?;
        let topic = QueueName::checked(text(command, "topic")?)?;
        let payload = command
            .get("payload")
            .filter(|payload| !payload.is_null())
            .ok_or_else(|| failure("Publishing requires a payload."))?;
        let mut request = json!({ "topic": topic, "payload": payload });
        if let Some(key) = Self::idempotency_key(command)? {
            request["idempotencyKey"] = json!(key);
        }
        let delay = Self::bounded(command, "delayMs", 0, 0..=Self::MAX_DELAY_MS)?;
        request["delayMs"] = json!(delay);
        let answer = self
            .store
            .call("queuePublish", request)
            .await
            .map_err(Self::queue_failure)?;
        Ok(answer.get("id").cloned().unwrap_or(Value::Null))
    }

    async fn claim(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::CLAIM_KEYS)?;
        let request = json!({
            "topic": QueueName::checked(text(command, "topic")?)?,
            "group": QueueName::checked(text(command, "group")?)?,
            "maxMessages": Self::bounded(command, "maxMessages", 1, 1..=Self::MAX_MESSAGES)?,
            "visibilityTimeoutMs": Self::bounded(command, "leaseMs", Self::DEFAULT_LEASE_MS, 1..=Self::MAX_LEASE_MS)?,
            "maxAttempts": Self::bounded(command, "maxAttempts", Self::DEFAULT_ATTEMPTS, 1..=Self::MAX_ATTEMPTS)?,
        });
        let answer = self
            .store
            .call("queueClaim", request)
            .await
            .map_err(Self::queue_failure)?;
        let deliveries = answer.as_array().cloned().unwrap_or_default();
        let mut claimed = Vec::with_capacity(deliveries.len());
        for delivery in deliveries {
            claimed.push(Self::delivery(&delivery));
        }
        Ok(Value::Array(claimed))
    }

    async fn settle(&self, command: &Value, operation: &str) -> Result<Value, Failure> {
        object_keys(command, &Self::SETTLE_KEYS)?;
        let mut request = json!({
            "topic": QueueName::checked(text(command, "topic")?)?,
            "group": QueueName::checked(text(command, "group")?)?,
            "id": Self::token(command, "id")?,
            "receipt": Self::token(command, "receipt")?,
        });
        if operation == "queueNack" {
            request["delayMs"] = json!(Self::bounded(
                command,
                "delayMs",
                0,
                0..=Self::MAX_DELAY_MS
            )?);
        } else if command.get("delayMs").is_some() {
            return Err(failure("Only a negative acknowledgement takes a delay."));
        }
        self.store
            .call(operation, request)
            .await
            .map_err(Self::queue_failure)?;
        Ok(json!(true))
    }

    fn delivery(claimed: &Value) -> Value {
        let mut delivery = json!({});
        for field in Self::DELIVERY_FIELDS {
            delivery[field] = claimed.get(field).cloned().unwrap_or(Value::Null);
        }
        delivery
    }

    fn idempotency_key(command: &Value) -> Result<Option<&str>, Failure> {
        let Some(key) = command
            .get("key")
            .filter(|key| !key.is_null() && key.as_str() != Some(""))
        else {
            return Ok(None);
        };
        let key = key
            .as_str()
            .filter(|key| key.len() <= Self::MAX_KEY_LENGTH)
            .ok_or_else(|| failure("An idempotency key is text of at most 256 bytes."))?;
        Ok(Some(key))
    }

    fn token<'command>(command: &'command Value, key: &str) -> Result<&'command str, Failure> {
        command
            .get(key)
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty() && token.len() <= Self::MAX_TOKEN_LENGTH)
            .filter(|token| token.bytes().all(|byte| byte.is_ascii_alphanumeric()))
            .ok_or_else(|| {
                failure("Acknowledging requires the id and receipt of a claimed delivery.")
            })
    }

    fn bounded(
        command: &Value,
        key: &str,
        default: u64,
        allowed: RangeInclusive<u64>,
    ) -> Result<u64, Failure> {
        let Some(value) = command.get(key).filter(|value| !value.is_null()) else {
            return Ok(default);
        };
        value
            .as_u64()
            .filter(|value| allowed.contains(value))
            .ok_or_else(|| failure("A queue number is a whole number inside its documented bound."))
    }

    fn queue_failure(error: FyloError) -> Failure {
        if error.code == "EQUEUE_RECEIPT" {
            return failure(
                "The queue receipt is stale; the lease expired or another worker settled this delivery.",
            );
        }
        error.into_failure()
    }
}
