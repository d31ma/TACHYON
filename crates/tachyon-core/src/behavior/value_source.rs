use super::{Input, Marker};
use crate::Failure;
use serde_json::{Map, Value};

pub(super) struct ValueSource;

impl ValueSource {
    pub(super) const NAME: &'static str = "ValueSource";
    const DATA_KEY: &'static str = "caseInputs";
    const CASE_PREFIX: &'static str = "test";
    const MAX_INPUTS: usize = 256;
    const NOT_A_CASE: &'static str =
        "ValueSource goes on a test case: a method whose name begins with `test`.";
    const NOT_A_LIST: &'static str = "ValueSource takes one list of 1 to 256 literal inputs, for example ValueSource([1, 2, 3]).";
    const REPEATED: &'static str = "A test case has at most one ValueSource.";
    const NOT_SCALAR: &'static str = "In Rust, ValueSource lists numbers, strings or booleans, for example #[ValueSource([1, 2, 3])].";

    pub(super) fn record(
        input: &Input<'_>,
        marker: &Marker,
        method: &str,
        data: &mut Map<String, Value>,
    ) -> Result<(), Failure> {
        Self::require_case(input, marker, method)?;
        let values = Self::values(input, marker)?;
        let table = Self::table(input, marker, data)?;
        let previous = table.insert(method.to_owned(), Value::Array(values));
        Self::require_first(input, marker, previous.as_ref())
    }

    pub(super) fn record_scalars(
        input: &Input<'_>,
        marker: &Marker,
        method: &str,
        data: &mut Map<String, Value>,
    ) -> Result<(), Failure> {
        let values = Self::values(input, marker)?;
        Self::require_scalars(input, marker, &values)?;
        Self::record(input, marker, method, data)
    }

    pub(super) fn inputs(data: &Map<String, Value>) -> Map<String, Value> {
        data.get(Self::DATA_KEY)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
    }

    pub(super) fn case_table(cases: &[String], inputs: &Map<String, Value>) -> Value {
        let rows = cases
            .iter()
            .flat_map(|method| Self::method_rows(method, inputs.get(method)))
            .collect();
        Value::Array(rows)
    }

    pub(super) fn rust_calls(
        cases: &[String],
        inputs: &Map<String, Value>,
    ) -> Vec<(String, String)> {
        cases
            .iter()
            .flat_map(|method| Self::method_calls(method, inputs.get(method)))
            .collect()
    }

    fn require_case(input: &Input<'_>, marker: &Marker, method: &str) -> Result<(), Failure> {
        if method.starts_with(Self::CASE_PREFIX) {
            return Ok(());
        }
        Err(input.failure(marker.span.start, Self::NOT_A_CASE))
    }

    fn values(input: &Input<'_>, marker: &Marker) -> Result<Vec<Value>, Failure> {
        let listed = Self::list(input, marker)?;
        Self::bounded(input, marker, listed)
    }

    fn list(input: &Input<'_>, marker: &Marker) -> Result<Vec<Value>, Failure> {
        let parsed = super::literal::parse(&marker.arguments).ok();
        let Some(Value::Array(values)) = parsed else {
            return Err(input.failure(marker.span.start, Self::NOT_A_LIST));
        };
        Ok(values)
    }

    fn bounded(
        input: &Input<'_>,
        marker: &Marker,
        values: Vec<Value>,
    ) -> Result<Vec<Value>, Failure> {
        if (1..=Self::MAX_INPUTS).contains(&values.len()) {
            return Ok(values);
        }
        Err(input.failure(marker.span.start, Self::NOT_A_LIST))
    }

    fn require_scalars(
        input: &Input<'_>,
        marker: &Marker,
        values: &[Value],
    ) -> Result<(), Failure> {
        if values.iter().all(Self::is_scalar) {
            return Ok(());
        }
        Err(input.failure(marker.span.start, Self::NOT_SCALAR))
    }

    fn is_scalar(value: &Value) -> bool {
        matches!(value, Value::Number(_) | Value::String(_) | Value::Bool(_))
    }

    fn table<'d>(
        input: &Input<'_>,
        marker: &Marker,
        data: &'d mut Map<String, Value>,
    ) -> Result<&'d mut Map<String, Value>, Failure> {
        data.entry(Self::DATA_KEY)
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| input.failure(marker.span.start, Self::NOT_A_LIST))
    }

    fn require_first(
        input: &Input<'_>,
        marker: &Marker,
        previous: Option<&Value>,
    ) -> Result<(), Failure> {
        if previous.is_none() {
            return Ok(());
        }
        Err(input.failure(marker.span.start, Self::REPEATED))
    }

    fn method_rows(method: &str, values: Option<&Value>) -> Vec<Value> {
        let Some(values) = values.and_then(Value::as_array) else {
            return vec![serde_json::json!([method, method, []])];
        };
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                serde_json::json!([Self::case_name(method, index), method, [value]])
            })
            .collect()
    }

    fn method_calls(method: &str, values: Option<&Value>) -> Vec<(String, String)> {
        let Some(values) = values.and_then(Value::as_array) else {
            return vec![(method.to_owned(), format!("self.{method}()"))];
        };
        Self::input_calls(method, values)
    }

    fn input_calls(method: &str, values: &[Value]) -> Vec<(String, String)> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let call = format!("self.{method}({})", Self::rust_literal(value));
                (Self::case_name(method, index), call)
            })
            .collect()
    }

    fn case_name(method: &str, index: usize) -> String {
        format!("{method}[{}]", index + 1)
    }

    fn rust_literal(value: &Value) -> String {
        match value {
            Value::String(text) => format!("{text:?}"),
            other => other.to_string(),
        }
    }
}
