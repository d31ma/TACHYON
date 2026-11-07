use super::Term;
use serde_json::{Value, json};

pub(super) struct CrudQueries;

impl CrudQueries {
    pub(super) const FIND_ALL: [&'static str; 2] = ["findAll", "find_all"];
    const FIND_BY_ID: [&'static str; 2] = ["findById", "find_by_id"];
    const ID_TAILS: [&'static str; 2] = ["Id", "id"];
    const FIND_ONE: &'static str = "findOne";
    pub(super) const OPERATIONS: [&'static str; 5] =
        ["find", "findOne", "count", "exists", "delete"];

    pub(super) fn find_all(name: &str, arity: usize) -> Option<Value> {
        let page = (arity == 1).then_some(0);
        Self::FIND_ALL
            .contains(&name)
            .then(|| json!({"operation":"find","where":[],"pageArgument":page,"orderBy":[]}))
    }

    pub(super) fn operation(name: &str, operation: &'static str) -> &'static str {
        [operation, Self::FIND_ONE][usize::from(Self::FIND_BY_ID.contains(&name))]
    }

    pub(super) fn id_terms(tail: &str, id: &Value) -> Option<Vec<Term>> {
        let field = id.as_str().filter(|_| Self::ID_TAILS.contains(&tail))?;
        Some(vec![Term {
            field: field.to_owned(),
            operator: "eq",
        }])
    }
}
