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
    const ACK_KEYS: [&'static str; 6] = ["operation", "command", "topic", "group", "id", "receipt"];
    const NACK_KEYS: [&'static str; 7] = [
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

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        self.run(text(command, "command")?, command).await
    }

    pub(super) fn new(store: &'store FyloStore) -> Self {
        Self { store }
    }

    async fn run(&self, name: &str, command: &Value) -> Result<Value, Failure> {
        match name {
            "claim" => self.claim(command).await,
            "ack" => self.ack(command).await,
            "nack" => self.nack(command).await,
            "publish" => self.publish(command).await,
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
        let delay = Self::bounded(command, "delayMs", 0, 0..=Self::MAX_DELAY_MS)?;
        let request = json!({ "topic": topic, "payload": payload, "delayMs": delay });
        let request = Self::with_key(request, Self::idempotency_key(command)?);
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
        Ok(Value::Array(
            deliveries.iter().map(Self::delivery).collect(),
        ))
    }

    async fn ack(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::ACK_KEYS)?;
        let request = Self::receipt_request(command)?;
        self.settle("queueAck", request).await
    }

    async fn nack(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(command, &Self::NACK_KEYS)?;
        let mut request = Self::receipt_request(command)?;
        request["delayMs"] = json!(Self::bounded(
            command,
            "delayMs",
            0,
            0..=Self::MAX_DELAY_MS
        )?);
        self.settle("queueNack", request).await
    }

    async fn settle(&self, operation: &str, request: Value) -> Result<Value, Failure> {
        self.store
            .call(operation, request)
            .await
            .map_err(Self::queue_failure)?;
        Ok(json!(true))
    }

    fn receipt_request(command: &Value) -> Result<Value, Failure> {
        Ok(json!({
            "topic": QueueName::checked(text(command, "topic")?)?,
            "group": QueueName::checked(text(command, "group")?)?,
            "id": Self::token(command, "id")?,
            "receipt": Self::token(command, "receipt")?,
        }))
    }

    fn delivery(claimed: &Value) -> Value {
        let mut delivery = json!({});
        for field in Self::DELIVERY_FIELDS {
            delivery[field] = claimed.get(field).cloned().unwrap_or(Value::Null);
        }
        delivery
    }

    fn idempotency_key(command: &Value) -> Result<Option<String>, Failure> {
        let key = command.get("key").cloned().unwrap_or(Value::Null);
        let text = key.as_str().unwrap_or_default().to_owned();
        let checks = [
            [key.is_null(), key.is_string()].contains(&true),
            text.len() <= Self::MAX_KEY_LENGTH,
        ];
        let valid = checks == [true, true];
        let named = (!text.is_empty()).then_some(text);
        valid
            .then_some(named)
            .ok_or_else(|| failure("An idempotency key is text of at most 256 bytes."))
    }

    fn with_key(mut request: Value, key: Option<String>) -> Value {
        if let Some(key) = key {
            request["idempotencyKey"] = json!(key);
        }
        request
    }

    fn token<'command>(command: &'command Value, key: &str) -> Result<&'command str, Failure> {
        let token = command.get(key).and_then(Value::as_str).unwrap_or_default();
        let checks = [
            (1..=Self::MAX_TOKEN_LENGTH).contains(&token.len()),
            token.bytes().all(|byte| byte.is_ascii_alphanumeric()),
        ];
        let valid = checks == [true, true];
        valid.then_some(token).ok_or_else(|| {
            failure("Acknowledging requires the id and receipt of a claimed delivery.")
        })
    }

    fn bounded(
        command: &Value,
        key: &str,
        default: u64,
        allowed: RangeInclusive<u64>,
    ) -> Result<u64, Failure> {
        let value = command.get(key).filter(|value| !value.is_null());
        value.map_or(Ok(default), |value| Self::number_in(value, &allowed))
    }

    fn number_in(value: &Value, allowed: &RangeInclusive<u64>) -> Result<u64, Failure> {
        value
            .as_u64()
            .filter(|number| allowed.contains(number))
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
