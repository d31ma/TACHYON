use super::{JsonFields, ModelDifference, RequestFields, SchemaChange};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) struct ApiDifference;

impl ApiDifference {
    const ROUTES: &'static str = "routes";
    const ROUTE: &'static str = "route";
    const METHODS: &'static str = "methods";
    const STATUS: &'static str = "status";
    const REQUEST: &'static str = "request";
    const BODY: &'static str = "body";
    const RESULT: &'static str = "ok";

    pub(super) fn breaks(old: &Value, new: &Value) -> Vec<String> {
        let mut found = Self::route_breaks(old, new);
        found.extend(ModelDifference::breaks(old, new));
        found
    }

    fn route_breaks(old: &Value, new: &Value) -> Vec<String> {
        let current = Self::routes(new);
        Self::routes(old)
            .into_iter()
            .flat_map(|(route, method)| {
                Self::one_route_breaks(&route, method, current.get(&route).copied())
            })
            .collect()
    }

    fn routes(api: &Value) -> BTreeMap<String, &Value> {
        api.get(Self::ROUTES)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|route| (Self::route_name(route), route))
            .collect()
    }

    fn route_name(route: &Value) -> String {
        let name = route.get(Self::ROUTE).and_then(Value::as_str);
        String::from(name.unwrap_or_default())
    }

    fn one_route_breaks(route: &str, old: &Value, new: Option<&Value>) -> Vec<String> {
        new.map_or_else(
            || vec![format!("the route '{route}' was removed")],
            |new| Self::method_breaks(route, old, new),
        )
    }

    fn method_breaks(route: &str, old: &Value, new: &Value) -> Vec<String> {
        let current = JsonFields::object(new, Self::METHODS);
        JsonFields::object(old, Self::METHODS)
            .iter()
            .flat_map(|(verb, method)| {
                let at = format!("{verb} {route}");
                Self::one_method_breaks(&at, method, current.get(verb))
            })
            .collect()
    }

    fn one_method_breaks(at: &str, old: &Value, new: Option<&Value>) -> Vec<String> {
        new.map_or_else(
            || vec![format!("{at} was removed")],
            |new| Self::present_method_breaks(at, old, new),
        )
    }

    fn present_method_breaks(at: &str, old: &Value, new: &Value) -> Vec<String> {
        let mut found = Self::status_breaks(at, old, new);
        found.extend(Self::body_breaks(
            at,
            &old[Self::REQUEST],
            &new[Self::REQUEST],
        ));
        found.extend(Self::request_breaks(
            at,
            &old[Self::REQUEST],
            &new[Self::REQUEST],
        ));
        found.extend(Self::result_breaks(at, old, new));
        found
    }

    fn status_breaks(at: &str, old: &Value, new: &Value) -> Vec<String> {
        let message = format!(
            "{at} answers with status {} instead of {}",
            new[Self::STATUS],
            old[Self::STATUS]
        );
        (old[Self::STATUS] != new[Self::STATUS])
            .then_some(message)
            .into_iter()
            .collect()
    }

    fn body_breaks(at: &str, old: &Value, new: &Value) -> Vec<String> {
        let old_body = &old[Self::BODY];
        let new_body = &new[Self::BODY];
        let message = format!("{at} reads a request body that it did not accept before");
        (!SchemaChange::accepts(new_body, old_body) & !new_body.is_null())
            .then_some(message)
            .into_iter()
            .collect()
    }

    fn request_breaks(at: &str, old: &Value, new: &Value) -> Vec<String> {
        let old_fields = RequestFields::of(old).fields;
        let new_fields = RequestFields::of(new).fields;
        let mut found: Vec<String> = old_fields
            .iter()
            .filter_map(|(name, old_field)| {
                Self::old_field_break(at, name, *old_field, new_fields.get(name))
            })
            .collect();
        found.extend(
            new_fields
                .iter()
                .filter(|(name, (required, _))| *required & !old_fields.contains_key(*name))
                .map(|(name, _)| format!("{at} needs the new {name}")),
        );
        found
    }

    fn old_field_break(
        at: &str,
        name: &str,
        old: (bool, &Value),
        new: Option<&(bool, &Value)>,
    ) -> Option<String> {
        new.map_or_else(
            || Some(format!("{at} no longer reads the {name}")),
            |new| Self::changed_field_break(at, name, old, *new),
        )
    }

    fn changed_field_break(
        at: &str,
        name: &str,
        old: (bool, &Value),
        new: (bool, &Value),
    ) -> Option<String> {
        let narrowed = !SchemaChange::accepts(new.1, old.1);
        let now_required = new.0 && !old.0;
        let message = format!("{at} narrowed the {name} or made it required");
        (narrowed | now_required).then_some(message)
    }

    fn result_breaks(at: &str, old: &Value, new: &Value) -> Vec<String> {
        let old_result = &old[Self::RESULT];
        let new_result = &new[Self::RESULT];
        let message = format!("{at} answers with a value that callers did not expect before");
        (!old_result.is_null() & !SchemaChange::accepts(old_result, new_result))
            .then_some(message)
            .into_iter()
            .collect()
    }
}
