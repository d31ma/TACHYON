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
        if self.conditions.is_empty() {
            return json!({});
        }
        json!({ "$ops": [Value::Object(self.conditions.clone())] })
    }

    pub(super) fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    pub(super) fn size(&self) -> usize {
        self.size
    }

    fn parse_conditions(command: &Value) -> Result<Map<String, Value>, Failure> {
        let mut conditions = Map::new();
        let Some(criteria) = command.get("where").filter(|criteria| !criteria.is_null()) else {
            return Ok(conditions);
        };
        let criteria = criteria
            .as_object()
            .filter(|criteria| criteria.len() <= Self::MAX_WHERE_FIELDS)
            .ok_or_else(|| failure("A scroll filter is an object of at most 8 field values."))?;
        for (field, value) in criteria {
            scalar(value)?;
            let field = identifier(field)?;
            conditions.insert(field.to_owned(), json!({ "$eq": value }));
        }
        Ok(conditions)
    }

    fn parse_cursor(command: &Value) -> Result<Option<String>, Failure> {
        let Some(cursor) = command.get("cursor").filter(|cursor| !cursor.is_null()) else {
            return Ok(None);
        };
        let cursor = cursor
            .as_str()
            .filter(|cursor| !cursor.is_empty() && cursor.len() <= Self::MAX_CURSOR_LENGTH)
            .filter(|cursor| cursor.bytes().all(|byte| byte.is_ascii_graphic()))
            .ok_or_else(|| {
                failure("A scroll cursor is the opaque text that the previous page returned.")
            })?;
        Ok(Some(cursor.to_owned()))
    }

    fn parse_size(command: &Value) -> Result<usize, Failure> {
        let Some(size) = command.get("size").filter(|size| !size.is_null()) else {
            return Ok(Self::DEFAULT_SIZE);
        };
        size.as_u64()
            .and_then(|size| usize::try_from(size).ok())
            .filter(|size| *size > 0 && *size <= ManagedStorage::MAX_ROWS)
            .ok_or_else(|| failure("A scroll page size is from 1 to 100."))
    }
}
