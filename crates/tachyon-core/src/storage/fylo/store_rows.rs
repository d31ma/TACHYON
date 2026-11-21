use serde_json::Value;

use crate::Failure;
use crate::storage::{ManagedStorage, failure};

pub(super) struct StoreRows;

impl StoreRows {
    const UNEXPECTED: &'static str = "FYLO returned an unexpected Store answer shape.";

    pub(super) fn found(answer: Value) -> Result<Vec<Value>, Failure> {
        let rows = Self::entities(answer)?;
        Self::refuse_oversized(&rows)?;
        Ok(rows)
    }

    pub(super) fn entity(answer: Value) -> Result<Value, Failure> {
        Self::checked_entity(answer)
    }

    pub(super) fn scroll_page(answer: &Value) -> Result<Value, Failure> {
        let content = Self::found(answer["content"].clone())?;
        let next = Self::cursor(&answer["nextCursor"])?;
        Ok(serde_json::json!({ "content": content, "nextCursor": next }))
    }

    fn entities(answer: Value) -> Result<Vec<Value>, Failure> {
        match answer {
            Value::Array(rows) => Self::checked_entities(rows),
            _ => Err(failure(Self::UNEXPECTED)),
        }
    }

    fn checked_entities(rows: Vec<Value>) -> Result<Vec<Value>, Failure> {
        let mut entities = Vec::with_capacity(rows.len());
        for row in rows {
            entities.push(Self::checked_entity(row)?);
        }
        Ok(entities)
    }

    fn checked_entity(answer: Value) -> Result<Value, Failure> {
        match answer {
            Value::Object(_) => Ok(answer),
            _ => Err(failure(Self::UNEXPECTED)),
        }
    }

    fn cursor(next: &Value) -> Result<Value, Failure> {
        match next {
            Value::Null | Value::String(_) => Ok(next.clone()),
            _ => Err(failure(Self::UNEXPECTED)),
        }
    }

    fn refuse_oversized(rows: &[Value]) -> Result<(), Failure> {
        let bytes = serde_json::to_vec(rows).map_or(usize::MAX, |encoded| encoded.len());
        if bytes > ManagedStorage::MAX_BYTES {
            return Err(failure(
                "FYLO result exceeds the byte limit; paginate the query.",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::StoreRows;

    #[test]
    fn a_list_of_entities_is_the_rows() {
        let rows = StoreRows::found(json!([{"sku": "4W1", "qty": 2}])).ok();
        assert_eq!(rows, Some(vec![json!({"sku": "4W1", "qty": 2})]));
    }

    #[test]
    fn an_answer_that_is_not_a_list_of_objects_is_refused() {
        assert!(StoreRows::found(json!({"items": []})).is_err());
        assert!(StoreRows::found(json!([1])).is_err());
    }

    #[test]
    fn a_scroll_page_keeps_its_content_and_next_cursor() {
        let page = json!({"content": [{"id": "4W1"}], "nextCursor": "c.1"});
        assert_eq!(StoreRows::scroll_page(&page).ok(), Some(page));
        let last = json!({"content": [], "nextCursor": null});
        assert_eq!(StoreRows::scroll_page(&last).ok(), Some(last));
    }

    #[test]
    fn a_scroll_page_with_a_strange_cursor_is_refused() {
        let page = json!({"content": [], "nextCursor": 7});
        assert!(StoreRows::scroll_page(&page).is_err());
    }

    #[test]
    fn a_result_over_one_mebibyte_is_refused() {
        let large = "x".repeat(1024 * 1024);
        assert!(StoreRows::found(json!([{"id": "4W1", "text": large}])).is_err());
    }
}
