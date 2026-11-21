use serde_json::{Value, json};

use super::identifier;
use crate::Failure;
use crate::storage::dialect::sort_field_and_direction;
use crate::storage::{ManagedStorage, failure, object_keys, scalar, text};

pub(super) struct StoreQuery<'entity> {
    id_field: &'entity str,
}

impl<'entity> StoreQuery<'entity> {
    const PREDICATE_KEYS: [&'static str; 3] = ["field", "operator", "value"];
    const PAGE_KEYS: [&'static str; 4] = ["page", "size", "sort", "offset"];
    const COMPARISONS: [&'static str; 6] = ["eq", "ne", "lt", "lte", "gt", "gte"];
    const NULL_CHECKS: [&'static str; 2] = ["isNull", "isNotNull"];
    const IN: &'static str = "in";
    const ASCENDING: &'static str = "asc";
    const DESCENDING: &'static str = "desc";
    const MAX_PREDICATES: usize = 8;
    const MAX_IN_VALUES: usize = 100;
    const MAX_SORT_KEYS: usize = 5;
    const MAX_PAGE_INDEX: u64 = 1_000_000;
    const DEFAULT_PAGE_SIZE: u64 = 20;
    const MAX_QUERY_BYTES: usize = 64 * 1024;
    pub(super) const TOO_LARGE: &'static str =
        "The query is larger than FYLO's 64 KiB query limit; use fewer or shorter values.";

    pub(super) fn new(id_field: &'entity str) -> Self {
        Self { id_field }
    }

    pub(super) fn conditions(query: &Value) -> Result<Vec<Value>, Failure> {
        let predicates = Self::predicates(query)?;
        Self::converted_conditions(predicates)
    }

    pub(super) fn matches_nothing(conditions: &[Value]) -> bool {
        Self::has_empty_in_list(conditions)
    }

    pub(super) fn order(&self, page: &Value) -> Result<Vec<Value>, Failure> {
        let items = Self::sort_items(page)?;
        let order = Self::sort_keys(items)?;
        Ok(self.without_trailing_id(order))
    }

    pub(super) fn bounded(query: Value) -> Result<Value, Failure> {
        let bytes = serde_json::to_vec(&query).map_or(usize::MAX, |encoded| encoded.len());
        Self::refuse_large(bytes)?;
        Ok(query)
    }

    pub(super) fn page(page: &Value) -> Result<Value, Failure> {
        object_keys(page, &Self::PAGE_KEYS)?;
        let index = Self::page_index(page)?;
        let size = Self::page_size(page)?;
        let offset = index * size;
        Self::refuse_other_offset(page, offset)?;
        Ok(json!({ "size": size, "offset": offset }))
    }

    fn predicates(query: &Value) -> Result<&Vec<Value>, Failure> {
        query["where"]
            .as_array()
            .filter(|items| items.len() <= Self::MAX_PREDICATES)
            .ok_or_else(|| failure("Store query requires bounded predicates."))
    }

    fn converted_conditions(predicates: &[Value]) -> Result<Vec<Value>, Failure> {
        let mut conditions = Vec::with_capacity(predicates.len());
        for predicate in predicates {
            conditions.push(Self::condition(predicate)?);
        }
        Ok(conditions)
    }

    fn has_empty_in_list(conditions: &[Value]) -> bool {
        conditions
            .iter()
            .any(|condition| condition["value"].as_array().is_some_and(Vec::is_empty))
    }

    fn sort_items(page: &Value) -> Result<&Vec<Value>, Failure> {
        static NO_SORT: Vec<Value> = Vec::new();
        page.get("sort")
            .map_or(Some(&NO_SORT), Value::as_array)
            .filter(|items| items.len() <= Self::MAX_SORT_KEYS)
            .ok_or_else(|| failure("Invalid repository sorting."))
    }

    fn sort_keys(items: &[Value]) -> Result<Vec<Value>, Failure> {
        let mut order = Vec::with_capacity(items.len());
        for item in items {
            order.push(Self::sort_key(item)?);
        }
        Ok(order)
    }

    fn condition(predicate: &Value) -> Result<Value, Failure> {
        object_keys(predicate, &Self::PREDICATE_KEYS)?;
        let field = identifier(text(predicate, "field")?)?;
        let operator = text(predicate, "operator")?;
        let mut condition = json!({ "field": field, "operator": operator });
        if let Some(value) = Self::checked_value(predicate, operator)? {
            condition["value"] = value;
        }
        Ok(condition)
    }

    fn checked_value(predicate: &Value, operator: &str) -> Result<Option<Value>, Failure> {
        match operator {
            Self::IN => Self::in_values(predicate).map(Some),
            _ if Self::NULL_CHECKS.contains(&operator) => Ok(None),
            _ if Self::COMPARISONS.contains(&operator) => Self::comparison_value(predicate),
            _ => Err(failure("Unsupported repository comparison.")),
        }
    }

    fn in_values(predicate: &Value) -> Result<Value, Failure> {
        let values = predicate["value"]
            .as_array()
            .filter(|values| values.len() <= Self::MAX_IN_VALUES)
            .ok_or_else(|| failure("In predicate requires a bounded array."))?;
        for value in values {
            scalar(value)?;
        }
        Ok(Value::Array(values.clone()))
    }

    fn comparison_value(predicate: &Value) -> Result<Option<Value>, Failure> {
        let value = predicate
            .get("value")
            .ok_or_else(|| failure("Comparison predicate requires a value."))?;
        scalar(value)?;
        Ok(Some(value.clone()))
    }

    fn sort_key(item: &Value) -> Result<Value, Failure> {
        let (field, direction) = sort_field_and_direction(item)?;
        let known = [Self::ASCENDING, Self::DESCENDING].contains(&direction);
        if !known {
            return Err(failure("Invalid repository sort direction."));
        }
        let field = identifier(field)?;
        Ok(json!({ "field": field, "direction": direction }))
    }

    fn without_trailing_id(&self, mut order: Vec<Value>) -> Vec<Value> {
        let tie_breaker = json!({ "field": self.id_field, "direction": Self::ASCENDING });
        if order.last() == Some(&tie_breaker) {
            order.pop();
        }
        order
    }

    fn refuse_large(bytes: usize) -> Result<(), Failure> {
        if bytes > Self::MAX_QUERY_BYTES {
            return Err(failure(Self::TOO_LARGE));
        }
        Ok(())
    }

    fn page_index(page: &Value) -> Result<u64, Failure> {
        page.get("page")
            .map_or(Some(0), Value::as_u64)
            .filter(|index| *index <= Self::MAX_PAGE_INDEX)
            .ok_or_else(|| failure("Invalid repository page."))
    }

    fn page_size(page: &Value) -> Result<u64, Failure> {
        let largest = ManagedStorage::MAX_ROWS as u64;
        page.get("size")
            .map_or(Some(Self::DEFAULT_PAGE_SIZE), Value::as_u64)
            .filter(|size| (1..=largest).contains(size))
            .ok_or_else(|| failure("Invalid repository page size."))
    }

    fn refuse_other_offset(page: &Value, offset: u64) -> Result<(), Failure> {
        let mismatch = page
            .get("offset")
            .is_some_and(|declared| declared.as_u64() != Some(offset));
        if mismatch {
            return Err(failure("Invalid repository page offset."));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::StoreQuery;

    fn conditions(query: &Value) -> Value {
        let conditions = StoreQuery::conditions(query).unwrap_or_else(|error| panic!("{error}"));
        Value::Array(conditions)
    }

    fn order(sort: &Value) -> Value {
        let order = StoreQuery::new("sku")
            .order(&json!({ "sort": sort }))
            .unwrap_or_else(|error| panic!("{error}"));
        Value::Array(order)
    }

    #[test]
    fn predicates_pass_to_fylo_in_tachyons_own_shape() {
        let query = json!({"where": [
            {"field": "qty", "operator": "gte", "value": 3},
            {"field": "name", "operator": "in", "value": ["a", "b"]},
            {"field": "size", "operator": "isNull"}
        ]});
        assert_eq!(
            conditions(&query),
            json!([
                {"field": "qty", "operator": "gte", "value": 3},
                {"field": "name", "operator": "in", "value": ["a", "b"]},
                {"field": "size", "operator": "isNull"}
            ])
        );
    }

    #[test]
    fn the_entity_id_field_keeps_its_name_because_fylo_takes_the_id_field() {
        let query = json!({"where": [{"field": "sku", "operator": "eq", "value": "4W0"}]});
        assert_eq!(
            conditions(&query),
            json!([{"field": "sku", "operator": "eq", "value": "4W0"}])
        );
    }

    #[test]
    fn an_empty_in_list_matches_nothing() {
        let empty = [json!({"field": "name", "operator": "in", "value": []})];
        assert!(StoreQuery::matches_nothing(&empty));
        let filled = [json!({"field": "name", "operator": "in", "value": ["a"]})];
        assert!(!StoreQuery::matches_nothing(&filled));
    }

    #[test]
    fn unsafe_or_unbounded_predicates_are_refused() {
        let refused = [
            json!({"where": [{"field": "a;b", "operator": "eq", "value": 1}]}),
            json!({"where": [{"field": "qty", "operator": "like", "value": 1}]}),
            json!({"where": [{"field": "qty", "operator": "eq"}]}),
            json!({"where": [{"field": "qty", "operator": "eq", "value": {"x": 1}}]}),
            json!({"where": [{"field": "qty", "operator": "in", "value": vec![1; 101]}]}),
            json!({"where": vec![json!({"field": "qty", "operator": "isNull"}); 9]}),
        ];
        let accepted = refused
            .iter()
            .filter(|query| StoreQuery::conditions(query).is_ok())
            .count();
        assert_eq!(accepted, 0);
    }

    #[test]
    fn sort_keys_become_fylo_order_and_the_trailing_id_tie_breaker_is_left_to_fylo() {
        assert_eq!(
            order(&json!(["qty:desc", "sku:asc"])),
            json!([{"field": "qty", "direction": "desc"}])
        );
    }

    #[test]
    fn a_descending_id_sort_is_kept() {
        assert_eq!(
            order(&json!(["sku:desc"])),
            json!([{"field": "sku", "direction": "desc"}])
        );
    }

    #[test]
    fn a_query_over_sixty_four_kibibytes_is_refused_before_it_is_sent() {
        let large =
            json!({"where": [{"field": "name", "operator": "eq", "value": "x".repeat(70_000)}]});
        let refused = StoreQuery::bounded(large)
            .err()
            .map(|error| error.to_string());
        assert!(refused.unwrap_or_default().contains("64 KiB"));
        let small = json!({"where": []});
        assert_eq!(StoreQuery::bounded(small.clone()).ok(), Some(small));
    }

    #[test]
    fn a_page_is_a_size_and_an_offset_with_no_ten_thousand_bound() {
        let page = StoreQuery::page(&json!({"page": 100, "size": 100, "offset": 10_000}));
        assert_eq!(page.ok(), Some(json!({"size": 100, "offset": 10_000})));
    }

    #[test]
    fn an_empty_page_is_the_first_page_of_twenty() {
        let page = StoreQuery::page(&json!({}));
        assert_eq!(page.ok(), Some(json!({"size": 20, "offset": 0})));
    }

    #[test]
    fn a_page_whose_offset_disagrees_with_its_index_is_refused() {
        assert!(StoreQuery::page(&json!({"page": 2, "size": 10, "offset": 5})).is_err());
    }

    #[test]
    fn a_page_larger_than_one_hundred_rows_is_refused() {
        assert!(StoreQuery::page(&json!({"size": 101})).is_err());
    }
}
