use serde_json::{Map, Value, json};

use super::identifier;
use crate::Failure;
use crate::storage::{ManagedStorage, failure, object_keys, scalar};

pub(super) struct ScrollRequest {
    conditions: Map<String, Value>,
    cursor: Option<String>,
    size: usize,
}

impl ScrollRequest {
    const KEYS: [&'static str; 6] = [
        "operation",
        "command",
        "collection",
        "where",
        "cursor",
        "size",
    ];
    const DEFAULT_SIZE: usize = 20;
    const MAX_WHERE_FIELDS: usize = 8;
    const MAX_CURSOR_LENGTH: usize = 256;

    pub(super) fn parse(command: &Value) -> Result<Self, Failure> {
        object_keys(command, &Self::KEYS)?;
        Ok(Self {
            conditions: Self::parse_conditions(command)?,
            cursor: Self::parse_cursor(command)?,
            size: Self::parse_size(command)?,
        })
    }

    pub(super) fn filter(&self) -> Value {
        json!({ "$ops": [Value::Object(self.conditions.clone())] })
    }

    pub(super) fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    pub(super) fn size(&self) -> usize {
        self.size
    }

    fn present<'command>(command: &'command Value, key: &str) -> Option<&'command Value> {
        command.get(key).filter(|value| !value.is_null())
    }

    fn parse_conditions(command: &Value) -> Result<Map<String, Value>, Failure> {
        let criteria = Self::present(command, "where").map_or(Ok(Map::new()), Self::criteria)?;
        let mut conditions = Map::new();
        for (field, value) in &criteria {
            scalar(value)?;
            conditions.insert(identifier(field)?.to_owned(), json!({ "$eq": value }));
        }
        Ok(conditions)
    }

    fn criteria(value: &Value) -> Result<Map<String, Value>, Failure> {
        value
            .as_object()
            .filter(|criteria| criteria.len() <= Self::MAX_WHERE_FIELDS)
            .cloned()
            .ok_or_else(|| failure("A scroll filter is an object of at most 8 field values."))
    }

    fn parse_cursor(command: &Value) -> Result<Option<String>, Failure> {
        Self::present(command, "cursor")
            .map(Self::cursor_text)
            .transpose()
    }

    fn cursor_text(cursor: &Value) -> Result<String, Failure> {
        let text = cursor.as_str().unwrap_or_default();
        let checks = [
            (1..=Self::MAX_CURSOR_LENGTH).contains(&text.len()),
            text.bytes().all(|byte| byte.is_ascii_graphic()),
        ];
        let valid = checks == [true, true];
        valid.then(|| text.to_owned()).ok_or_else(|| {
            failure("A scroll cursor is the opaque text that the previous page returned.")
        })
    }

    fn parse_size(command: &Value) -> Result<usize, Failure> {
        Self::present(command, "size").map_or(Ok(Self::DEFAULT_SIZE), Self::size_number)
    }

    fn size_number(size: &Value) -> Result<usize, Failure> {
        let number = size
            .as_u64()
            .and_then(|size| usize::try_from(size).ok())
            .unwrap_or_default();
        let valid = (1..=ManagedStorage::MAX_ROWS).contains(&number);
        valid
            .then_some(number)
            .ok_or_else(|| failure("A scroll page size is from 1 to 100."))
    }
}
