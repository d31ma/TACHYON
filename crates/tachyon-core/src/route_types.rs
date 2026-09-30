use crate::yon::plan::{ParameterSource, Route};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod converters;
mod language_kinds;
mod mapping_models;
mod model_files;
mod php_doc;
mod type_text;
mod web_event_models;
mod web_event_sources;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod web_event_model_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod declared_name_tests;

pub(crate) use converters::{Converters, DartValues};
pub(crate) use mapping_models::MappingModels;
pub(crate) use php_doc::PhpDoc;
pub(crate) use type_text::TypeText;
pub(crate) use web_event_models::WebEventModels;
pub(crate) use web_event_sources::WebEventSources;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Shape {
    #[default]
    Any,
    Text,
    Integer,
    Number,
    Boolean,
    List(Box<Shape>),
    Optional(Box<Shape>),
    Model(String),
}

impl Shape {
    pub(crate) fn is_optional(&self) -> bool {
        matches!(self, Self::Optional(_))
    }

    fn inner(&self) -> &Self {
        match self {
            Self::Optional(inner) => inner,
            _ => self,
        }
    }

    fn plain_name(&self) -> String {
        match self {
            Self::Any => String::from("any JSON value"),
            Self::Text => String::from("text"),
            Self::Integer => String::from("a whole number"),
            Self::Number => String::from("a number"),
            Self::Boolean => String::from("true or false"),
            Self::List(item) => format!("a list of {}", item.plain_name()),
            Self::Optional(inner) => format!("{} or null", inner.plain_name()),
            Self::Model(name) => format!("a {name} object"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelField {
    pub name: String,
    pub shape: Shape,
    pub required: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Record {
    pub fields: Vec<ModelField>,
    pub bases: Vec<String>,
    pub open: bool,
    pub unreadable: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Model {
    Record(Record),
    Alias(Shape),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModelTable {
    models: BTreeMap<String, Model>,
    paths: BTreeMap<String, String>,
    topics: BTreeSet<String>,
}

impl ModelTable {
    const MAX_DEPTH: usize = 32;

    pub(crate) fn read(root: &Path, extension: &str) -> Result<Self, String> {
        Self::read_realm(root, "server", extension)
    }

    pub(crate) fn read_realm(root: &Path, realm: &str, extension: &str) -> Result<Self, String> {
        let mut files = Vec::new();
        for language in crate::fsharp_source::FsharpSource::model_languages(extension) {
            files.extend(model_files::ModelFolder::read_realm(root, realm, language)?);
        }
        Ok(Self::from_files(&files))
    }

    fn from_files(files: &[(String, String)]) -> Self {
        let mut table = Self::default();
        for (path, source) in files {
            let blanked = crate::topics::Topics::blanked(source);
            let language = model_files::ModelFolder::language(path);
            let models = model_files::ModelParser::models(language, &blanked);
            let declared: Vec<String> = models.iter().map(|(name, _)| name.clone()).collect();
            table
                .topics
                .extend(crate::topics::Topics::named(source, &declared));
            for (name, model) in models {
                table.paths.insert(name.clone(), path.clone());
                table.models.insert(name, model);
            }
        }
        table.join_bases();
        table
    }

    fn join_bases(&mut self) {
        let names: Vec<String> = self.models.keys().cloned().collect();
        for name in names {
            if let Some(Model::Record(record)) = self.models.get(&name) {
                let joined = self.joined(record, 0);
                self.models.insert(name, Model::Record(joined));
            }
        }
    }

    fn joined(&self, record: &Record, depth: usize) -> Record {
        let mut joined = Record {
            bases: Vec::new(),
            ..record.clone()
        };
        for base in &record.bases {
            match self.models.get(base) {
                Some(Model::Record(parent)) if depth < Self::MAX_DEPTH => {
                    let parent = self.joined(parent, depth + 1);
                    joined.open |= parent.open;
                    if joined.unreadable.is_none() {
                        joined.unreadable = parent.unreadable;
                    }
                    for field in parent.fields {
                        if !joined.fields.iter().any(|own| own.name == field.name) {
                            joined.fields.push(field);
                        }
                    }
                }
                _ => joined.open = true,
            }
        }
        joined
    }

    pub(crate) fn add_documented(&mut self, source: &str) {
        let models = model_files::ModelParser::jsdoc_models(source);
        let declared: Vec<String> = models.iter().map(|(name, _)| name.clone()).collect();
        self.topics
            .extend(crate::topics::Topics::named(source, &declared));
        for (name, model) in models {
            self.models.insert(name, model);
        }
    }

    pub(crate) fn add_web_events(&mut self) {
        self.models.extend(WebEventModels::models());
    }

    pub(crate) fn declared_names(extension: &str, source: &str) -> Vec<String> {
        Self::declared_models(extension, source)
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    pub(crate) fn declared_models(extension: &str, source: &str) -> Vec<(String, Model)> {
        let blanked = crate::topics::Topics::blanked(source);
        model_files::ModelParser::models(extension, &blanked)
    }

    pub(crate) fn topic_names(&self) -> Vec<String> {
        self.topics
            .iter()
            .filter(|name| self.is_record(name))
            .cloned()
            .collect()
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    pub(crate) fn record_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for (name, model) in &self.models {
            if matches!(model, Model::Record(_)) {
                names.push(name.clone());
            }
        }
        names
    }

    pub(crate) fn is_record(&self, name: &str) -> bool {
        matches!(self.models.get(name), Some(Model::Record(_)))
    }

    pub(crate) fn event_models(&self, name: &str) -> Result<Value, String> {
        if !self.is_record(name) {
            return Err(format!(
                "'{name}' is not a model. Declare it in the models folder as a class, record or struct with fields"
            ));
        }
        self.check_readable(&Shape::Model(name.to_owned()))?;
        let mut described = Map::new();
        let mut waiting = vec![name.to_owned()];
        while let Some(next) = waiting.pop() {
            if described.contains_key(&next) {
                continue;
            }
            let Some(Model::Record(record)) = self.models.get(&next) else {
                continue;
            };
            let mut fields = Map::new();
            for field in &record.fields {
                let mut text = self.shape_text(&field.shape, &mut waiting, 0);
                if field.required && !field.shape.is_optional() {
                    text.insert(0, '!');
                }
                fields.insert(field.name.clone(), Value::String(text));
            }
            described.insert(next, json!({"open": record.open, "fields": fields}));
        }
        Ok(Value::Object(described))
    }

    pub(crate) fn shape_description(
        &self,
        shape: &Shape,
        described: &mut Map<String, Value>,
    ) -> String {
        let mut waiting = Vec::new();
        let text = self.shape_text(shape, &mut waiting, 0);
        described.extend(self.models_of(&waiting));
        text
    }

    fn models_of(&self, names: &[String]) -> Map<String, Value> {
        names
            .iter()
            .flat_map(|name| self.model_entries(name))
            .collect()
    }

    fn model_entries(&self, name: &str) -> Map<String, Value> {
        self.event_models(name)
            .ok()
            .and_then(|described| described.as_object().cloned())
            .unwrap_or_default()
    }

    fn shape_text(&self, shape: &Shape, waiting: &mut Vec<String>, depth: usize) -> String {
        if depth > Self::MAX_DEPTH {
            return String::from("any");
        }
        match shape {
            Shape::Any => String::from("any"),
            Shape::Text => String::from("string"),
            Shape::Integer => String::from("integer"),
            Shape::Number => String::from("number"),
            Shape::Boolean => String::from("boolean"),
            Shape::Optional(inner) => format!("?{}", self.shape_text(inner, waiting, depth + 1)),
            Shape::List(item) => format!("[]{}", self.shape_text(item, waiting, depth + 1)),
            Shape::Model(name) => match self.models.get(name) {
                Some(Model::Record(_)) => {
                    waiting.push(name.clone());
                    name.clone()
                }
                Some(Model::Alias(alias)) => self.shape_text(alias, waiting, depth + 1),
                None => String::from("any"),
            },
        }
    }

    pub(crate) fn check_names(&self, shape: &Shape) -> Result<(), String> {
        match shape {
            Shape::List(inner) | Shape::Optional(inner) => self.check_names(inner),
            Shape::Model(name) if !self.contains(name) => Err(format!(
                "'{name}' is not a model. Declare it in server/models/, or use text, a number, true or false, or a list"
            )),
            _ => Ok(()),
        }
    }

    pub(crate) fn check_all(&self) -> Result<(), String> {
        for (name, model) in &self.models {
            if let Err(message) = self.check_model_names(model) {
                return Err(match self.paths.get(name) {
                    Some(path) => format!("Model '{name}' in '{path}': {message}"),
                    None => format!("Model '{name}': {message}"),
                });
            }
        }
        Ok(())
    }

    fn check_model_names(&self, model: &Model) -> Result<(), String> {
        match model {
            Model::Record(record) => {
                for field in &record.fields {
                    self.check_names(&field.shape)?;
                }
                Ok(())
            }
            Model::Alias(shape) => self.check_names(shape),
        }
    }

    pub(crate) fn check_readable(&self, shape: &Shape) -> Result<(), String> {
        let mut seen = Vec::new();
        self.check_readable_at(shape, &mut seen)
    }

    fn check_readable_at(&self, shape: &Shape, seen: &mut Vec<String>) -> Result<(), String> {
        match shape {
            Shape::List(inner) | Shape::Optional(inner) => self.check_readable_at(inner, seen),
            Shape::Model(name) if !seen.contains(name) => {
                seen.push(name.clone());
                let Some(Model::Record(record)) = self.models.get(name) else {
                    return Ok(());
                };
                if let Some(problem) = &record.unreadable {
                    return Err(format!(
                        "Model '{name}' cannot be read from a request: {problem}"
                    ));
                }
                for field in &record.fields {
                    self.check_readable_at(&field.shape, seen)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn check(&self, shape: &Shape, value: &Value, path: &str) -> Result<(), String> {
        self.check_at_depth(shape, value, path, 0)
    }

    fn check_at_depth(
        &self,
        shape: &Shape,
        value: &Value,
        path: &str,
        depth: usize,
    ) -> Result<(), String> {
        if depth > Self::MAX_DEPTH {
            return Err(format!("'{path}' is nested too deeply"));
        }
        let fits = match shape {
            Shape::Any => true,
            Shape::Text => value.is_string(),
            Shape::Integer => value.is_i64() || value.is_u64(),
            Shape::Number => value.is_number(),
            Shape::Boolean => value.is_boolean(),
            Shape::Optional(inner) => {
                return if value.is_null() {
                    Ok(())
                } else {
                    self.check_at_depth(inner, value, path, depth + 1)
                };
            }
            Shape::List(item) => return self.check_list(item, value, path, depth),
            Shape::Model(name) => return self.check_model(name, value, path, depth),
        };
        if fits {
            return Ok(());
        }
        Err(format!("'{path}' must be {}", shape.plain_name()))
    }

    fn check_list(
        &self,
        item: &Shape,
        value: &Value,
        path: &str,
        depth: usize,
    ) -> Result<(), String> {
        let Some(items) = value.as_array() else {
            return Err(format!("'{path}' must be a list"));
        };
        for (index, item_value) in items.iter().enumerate() {
            self.check_at_depth(item, item_value, &format!("{path}[{index}]"), depth + 1)?;
        }
        Ok(())
    }

    fn check_model(
        &self,
        name: &str,
        value: &Value,
        path: &str,
        depth: usize,
    ) -> Result<(), String> {
        let Some(model) = self.models.get(name) else {
            return Ok(());
        };
        let record = match model {
            Model::Alias(shape) => return self.check_at_depth(shape, value, path, depth + 1),
            Model::Record(record) => record,
        };
        let Some(object) = value.as_object() else {
            return Err(format!("'{path}' must be a {name} object"));
        };
        for key in object.keys() {
            if !record.open && !record.fields.iter().any(|field| field.name == *key) {
                return Err(format!("'{path}' contains a field not declared in {name}"));
            }
        }
        for field in &record.fields {
            let child = if path.is_empty() {
                field.name.clone()
            } else {
                format!("{path}.{}", field.name)
            };
            match object.get(&field.name) {
                Some(field_value) => {
                    self.check_at_depth(&field.shape, field_value, &child, depth + 1)?;
                }
                None if field.required && !field.shape.is_optional() => {
                    return Err(format!("'{child}' is required"));
                }
                None => {}
            }
        }
        Ok(())
    }

    pub(crate) fn schema(shape: &Shape) -> Value {
        match shape {
            Shape::Any => json!({}),
            Shape::Text => json!({"type": "string"}),
            Shape::Integer => json!({"type": "integer"}),
            Shape::Number => json!({"type": "number"}),
            Shape::Boolean => json!({"type": "boolean"}),
            Shape::List(item) => json!({"type": "array", "items": Self::schema(item)}),
            Shape::Optional(inner) => json!({"anyOf": [Self::schema(inner), {"type": "null"}]}),
            Shape::Model(name) => json!({"$ref": format!("#/models/{name}")}),
        }
    }

    pub(crate) fn model_schemas(&self) -> Value {
        let mut schemas = Map::new();
        for (name, model) in &self.models {
            schemas.insert(name.clone(), Self::model_schema(model));
        }
        Value::Object(schemas)
    }

    fn model_schema(model: &Model) -> Value {
        let record = match model {
            Model::Alias(shape) => return Self::schema(shape),
            Model::Record(record) => record,
        };
        let mut properties = Map::new();
        let mut required = Vec::new();
        for field in &record.fields {
            properties.insert(field.name.clone(), Self::schema(&field.shape));
            if field.required && !field.shape.is_optional() {
                required.push(Value::String(field.name.clone()));
            }
        }
        let mut schema = json!({"type": "object", "properties": properties, "required": required});
        if !record.open {
            schema["additionalProperties"] = Value::Bool(false);
        }
        schema
    }
}

pub(crate) struct TextValue;

impl TextValue {
    pub(crate) fn convert(shape: &Shape, text: &str) -> Option<Value> {
        match shape.inner() {
            Shape::Integer => text.parse::<i64>().ok().map(Value::from),
            Shape::Number => text
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .map(Value::from),
            Shape::Boolean => match text {
                "true" => Some(Value::Bool(true)),
                "false" => Some(Value::Bool(false)),
                _ => None,
            },
            Shape::List(_) | Shape::Model(_) => None,
            _ => Some(Value::String(text.to_owned())),
        }
    }

    pub(crate) fn plain_name(shape: &Shape) -> String {
        shape.inner().plain_name()
    }
}

pub(crate) struct RouteTypes;

impl RouteTypes {
    pub(crate) fn resolve(
        extension: &str,
        root: &Path,
        source: &str,
        routes: &mut [Route],
    ) -> Result<ModelTable, String> {
        let mut models = ModelTable::read(root, extension)?;
        if extension == "js" {
            models.add_documented(source);
        }
        models.check_all()?;
        let extension = crate::groovy_source::GroovySource::syntax(extension);
        let extension = crate::fsharp_source::FsharpSource::syntax(extension);
        let extension = crate::lua_source::LuaSource::syntax(extension);
        for route in routes.iter_mut() {
            Self::resolve_route(extension, &models, route)?;
        }
        Ok(models)
    }

    fn resolve_route(
        extension: &str,
        models: &ModelTable,
        route: &mut Route,
    ) -> Result<(), String> {
        for parameter in &mut route.parameters {
            if parameter.source == ParameterSource::Request {
                continue;
            }
            let text = if extension == "js" {
                Self::documented_type(&route.documentation, "@param", &parameter.code_name)
            } else {
                parameter.type_text.clone()
            };
            let shape = Self::shape(extension, models, &text).map_err(|message| {
                format!(
                    "The parameter '{}' of {}: {message}",
                    parameter.code_name, route.method
                )
            })?;
            Self::check_place(parameter.source, &parameter.code_name, &shape)?;
            if parameter.source == ParameterSource::Body {
                models.check_readable(&shape).map_err(|message| {
                    format!(
                        "The parameter '{}' of {}: {message}",
                        parameter.code_name, route.method
                    )
                })?;
            }
            parameter.shape = shape;
        }
        if route.socket.is_some() {
            return Self::check_socket_message(route);
        }
        if route.streaming {
            return Ok(());
        }
        let text = if extension == "js" {
            Self::documented_type(&route.documentation, "@returns", "")
        } else {
            route.returns.clone()
        };
        route.return_shape = Self::shape(extension, models, &text)
            .map_err(|message| format!("The return type of {}: {message}", route.method))?;
        for error in &mut route.error_responses {
            let text = if extension == "js" {
                Self::documented_type(&error.documentation, "@returns", "")
            } else {
                error.returns.clone()
            };
            error.return_shape = Self::shape(extension, models, &text)
                .map_err(|message| format!("The response for {}: {message}", error.error_type))?;
        }
        Ok(())
    }

    fn shape(extension: &str, models: &ModelTable, text: &str) -> Result<Shape, String> {
        let shape = TypeText::shape(extension, text)?;
        models.check_names(&shape)?;
        Ok(shape)
    }

    fn check_socket_message(route: &Route) -> Result<(), String> {
        let modelled = route
            .parameters
            .iter()
            .all(|parameter| matches!(parameter.shape.inner(), Shape::Model(_)));
        if modelled {
            return Ok(());
        }
        Err(format!(
            "The message parameter of {} needs a model from server/models/, such as ChatMessage, so Tachyon can check each message",
            route.method
        ))
    }

    fn check_place(source: ParameterSource, name: &str, shape: &Shape) -> Result<(), String> {
        if source == ParameterSource::Body {
            return Ok(());
        }
        let inner = shape.inner();
        let scalar = matches!(
            inner,
            Shape::Any | Shape::Text | Shape::Integer | Shape::Number | Shape::Boolean
        );
        let query_list = source == ParameterSource::Query
            && matches!(inner, Shape::List(item) if matches!(**item, Shape::Text | Shape::Integer | Shape::Number | Shape::Boolean));
        if scalar || query_list {
            return Ok(());
        }
        Err(format!(
            "The parameter '{name}' reads text from the request, so its type must be text, a number, or true or false. Use @FromBody for an object or a list"
        ))
    }

    pub(crate) fn documented_type(documentation: &str, tag: &str, name: &str) -> String {
        for line in documentation.lines() {
            let line = line.trim().trim_start_matches('*').trim();
            let Some(rest) = line.strip_prefix(tag).or_else(|| {
                (tag == "@returns")
                    .then(|| line.strip_prefix("@return"))
                    .flatten()
            }) else {
                continue;
            };
            let rest = rest.trim();
            let Some(after_brace) = rest.strip_prefix('{') else {
                continue;
            };
            let Some(close) = after_brace.rfind('}') else {
                continue;
            };
            let kind = after_brace[..close].trim();
            let written = after_brace[close + 1..]
                .split_whitespace()
                .next()
                .unwrap_or("");
            if name.is_empty() {
                return kind.to_owned();
            }
            let optional = written.starts_with('[');
            let written = written
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split('=')
                .next()
                .unwrap_or("");
            if written == name {
                if optional {
                    return format!("?{kind}");
                }
                return kind.to_owned();
            }
        }
        String::new()
    }
}
