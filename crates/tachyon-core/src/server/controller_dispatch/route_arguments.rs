use super::{Rejection, RouteRequest};
use crate::route_types::{ModelTable, Shape, TextValue};
use crate::yon::plan::{ParameterSource, Route, RouteParameter};
use axum::http::HeaderMap;
use axum::http::header::COOKIE;
use serde_json::Value;
use std::collections::BTreeMap;

pub(in crate::server) struct RouteArguments;

impl RouteArguments {
    pub(in crate::server) fn build(
        route: &Route,
        request: &RouteRequest<'_>,
        models: &ModelTable,
    ) -> Result<Vec<Value>, Rejection> {
        let query = Self::query_values(request.query);
        route
            .parameters
            .iter()
            .map(|parameter| Self::argument(parameter, request, &query, models))
            .collect()
    }

    fn argument(
        parameter: &RouteParameter,
        request: &RouteRequest<'_>,
        query: &BTreeMap<String, Vec<String>>,
        models: &ModelTable,
    ) -> Result<Value, Rejection> {
        match parameter.source {
            ParameterSource::Request => Ok(Value::Null),
            ParameterSource::Body => Self::body(request.body, &parameter.shape, models),
            ParameterSource::Query => Self::query(
                parameter,
                &query.get(&parameter.name).cloned().unwrap_or_default(),
            ),
            ParameterSource::Path => {
                Self::text(parameter, request.parameters.get(&parameter.name).cloned())
            }
            ParameterSource::Header => {
                Self::text(parameter, Self::header(request.headers, &parameter.name))
            }
            ParameterSource::Cookie => {
                Self::text(parameter, Self::cookie(request.headers, &parameter.name))
            }
        }
    }

    fn place(source: ParameterSource) -> &'static str {
        match source {
            ParameterSource::Path => "path",
            ParameterSource::Query => "query",
            ParameterSource::Header => "header",
            ParameterSource::Cookie => "cookie",
            ParameterSource::Body | ParameterSource::Request => "body",
        }
    }

    fn text(parameter: &RouteParameter, text: Option<String>) -> Result<Value, Rejection> {
        let place = Self::place(parameter.source);
        text.map_or_else(
            || Self::missing_text(parameter, place),
            |text| Self::converted_text(parameter, place, &text),
        )
    }

    fn missing_text(parameter: &RouteParameter, place: &'static str) -> Result<Value, Rejection> {
        if parameter.shape.is_optional() {
            return Ok(Value::Null);
        }
        let message = format!("The {place} value '{}' is required.", parameter.name);
        Err(Rejection::new(place, &parameter.name, &message))
    }

    fn converted_text(
        parameter: &RouteParameter,
        place: &'static str,
        text: &str,
    ) -> Result<Value, Rejection> {
        TextValue::convert(&parameter.shape, text).ok_or_else(|| {
            let message = format!(
                "The {place} value '{}' must be {}.",
                parameter.name,
                TextValue::plain_name(&parameter.shape)
            );
            Rejection::new(place, &parameter.name, &message)
        })
    }

    fn query(parameter: &RouteParameter, values: &[String]) -> Result<Value, Rejection> {
        Self::list_item(&parameter.shape).map_or_else(
            || Self::text(parameter, values.first().cloned()),
            |item| Self::query_list(parameter, item, values),
        )
    }

    fn list_item(shape: &Shape) -> Option<&Shape> {
        Self::direct_list_item(Self::without_optional(shape))
    }

    fn without_optional(shape: &Shape) -> &Shape {
        match shape {
            Shape::Optional(inner) => inner,
            other => other,
        }
    }

    fn direct_list_item(shape: &Shape) -> Option<&Shape> {
        match shape {
            Shape::List(item) => Some(item),
            _ => None,
        }
    }

    fn query_list(
        parameter: &RouteParameter,
        item: &Shape,
        values: &[String],
    ) -> Result<Value, Rejection> {
        values
            .iter()
            .map(|value| Self::query_item(parameter, item, value))
            .collect::<Result<Vec<Value>, Rejection>>()
            .map(Value::Array)
    }

    fn query_item(
        parameter: &RouteParameter,
        item: &Shape,
        value: &str,
    ) -> Result<Value, Rejection> {
        TextValue::convert(item, value).ok_or_else(|| {
            let message = format!(
                "Each query value '{}' must be {}.",
                parameter.name,
                TextValue::plain_name(item)
            );
            Rejection::new("query", &parameter.name, &message)
        })
    }

    fn query_values(query: Option<&str>) -> BTreeMap<String, Vec<String>> {
        let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let pairs = query
            .map(|query| url::form_urlencoded::parse(query.as_bytes()))
            .into_iter()
            .flatten();
        for (name, value) in pairs {
            values
                .entry(name.into_owned())
                .or_default()
                .push(value.into_owned());
        }
        values
    }

    fn header(headers: &HeaderMap, name: &str) -> Option<String> {
        let value = headers.get(name.to_ascii_lowercase().as_str())?;
        value.to_str().ok().map(str::to_owned)
    }

    fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
        headers
            .get_all(COOKIE)
            .iter()
            .filter_map(|header| header.to_str().ok())
            .flat_map(|text| text.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_owned())
    }

    fn body(body: &[u8], shape: &Shape, models: &ModelTable) -> Result<Value, Rejection> {
        let value = Self::parsed_body(body)?;
        Self::check_body_present(&value, shape)?;
        models.check(shape, &value, "body").map_err(|message| {
            Rejection::new(
                "body",
                "",
                &format!("The request body does not fit: {message}."),
            )
        })?;
        Ok(value)
    }

    fn parsed_body(body: &[u8]) -> Result<Value, Rejection> {
        if body.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(body)
            .map_err(|_| Rejection::new("body", "", "The request body is not valid JSON."))
    }

    fn check_body_present(value: &Value, shape: &Shape) -> Result<(), Rejection> {
        if value.is_null() && !shape.is_optional() && *shape != Shape::Any {
            return Err(Rejection::new("body", "", "The request body is required."));
        }
        Ok(())
    }
}
