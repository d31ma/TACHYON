use super::super::testing::{Unit, polyglot_unit};
use super::typed_answer::TypedAnswer;
use super::{Edit, Method, Module, Type, concrete_edit, render};
use serde_json::{Map, Value};
use std::fmt::Write as _;

pub(super) fn lower(module: &Module) -> Result<String, String> {
    let mut edits = Vec::new();
    let mut registrations = Vec::new();
    let mut helpers = String::new();
    for type_declaration in &module.types {
        edits.extend(concrete_edit(module, type_declaration));
        let data = type_data(module, type_declaration)?;
        if let Some(fields) = &type_declaration.constructor {
            edits.push(constructor_edit(module, type_declaration, fields));
        }
        if let Some(unit) = polyglot_unit(type_declaration) {
            let indent = body_indent(module, type_declaration);
            edits.push(Edit {
                range: type_declaration.body.start..type_declaration.body.start,
                replacement: test_members(&unit, &indent),
            });
            helpers = String::from(super::super::TestAssert::PYTHON);
        }
        registrations.push(registration(type_declaration, &data)?);
    }
    let suffix = format!(
        "{}{helpers}\nBehaviors = _BehaviorFactory({{{}}}, {:?}, {:?})\n",
        include_str!("python_runtime.py"),
        registrations.join(","),
        module.environment,
        module.module_id
    );
    render(module, edits, &suffix)
}

fn type_data(module: &Module, type_declaration: &Type) -> Result<Map<String, Value>, String> {
    let mut data = Map::new();
    data.extend(type_declaration.policies.clone());
    data.insert("name".into(), Value::String(type_declaration.name.clone()));
    data.insert(
        "configuration".into(),
        Value::Bool(type_declaration.configuration),
    );
    if let Some(value) = &type_declaration.cache {
        data.insert("cache".into(), value.clone());
    }
    if let Some(value) = &type_declaration.entity {
        data.insert("entity".into(), value.clone());
    }
    if let Some(value) = &type_declaration.table {
        data.insert("table".into(), Value::String(value.clone()));
    }
    let methods = method_data(module, type_declaration)?;
    data.insert("methods".into(), Value::Object(methods));
    if let Some(fields) = &type_declaration.constructor {
        data.insert("constructor".into(), serde_json::json!(fields));
    }
    Ok(data)
}

fn method_data(module: &Module, type_declaration: &Type) -> Result<Map<String, Value>, String> {
    let mut methods = Map::new();
    for method in &type_declaration.methods {
        if requires_async_def(method) && !method.asynchronous {
            return Err("Python Cached/WebCall/Retry/MaxAtOnce methods must use async def; Pageable-only preserves synchronous methods".into());
        }
        methods.insert(
            method.name.clone(),
            Value::Object(TypedAnswer::python_metadata(module, method)),
        );
    }
    Ok(methods)
}

fn requires_async_def(method: &Method) -> bool {
    method.metadata.contains_key("cacheable")
        || method.metadata.contains_key("cachePut")
        || method.metadata.contains_key("cacheEvict")
        || method.metadata.contains_key("query")
        || method.metadata.contains_key("httpExchange")
        || method.metadata.contains_key("retryable")
        || method.metadata.contains_key("concurrencyLimit")
}

fn constructor_edit(module: &Module, type_declaration: &Type, fields: &[String]) -> Edit {
    let indent = body_indent(module, type_declaration);
    let mut arguments = String::new();
    for name in fields {
        let _ = write!(arguments, ", {name}");
    }
    let mut constructor = format!("{indent}def __init__(self{arguments}):\n");
    for field in fields {
        let _ = writeln!(constructor, "{indent}{indent}self.{field} = {field}");
    }
    if fields.is_empty() {
        let _ = writeln!(constructor, "{indent}{indent}pass");
    }
    Edit {
        range: type_declaration.body.start..type_declaration.body.start,
        replacement: constructor,
    }
}

fn registration(type_declaration: &Type, data: &Map<String, Value>) -> Result<String, String> {
    let json = serde_json::to_string(data).map_err(|error| error.to_string())?;
    let quoted_json = serde_json::to_string(&json).map_err(|error| error.to_string())?;
    Ok(format!(
        "{}: _behavior_json.loads({quoted_json})",
        type_declaration.name
    ))
}

fn body_indent(module: &Module, type_declaration: &Type) -> String {
    let body = &module.source[type_declaration.body.clone()];
    let mut smallest_indent: Option<&str> = None;
    for line in body.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indent = &line[..line.len() - line.trim_start().len()];
        let is_smaller = smallest_indent.is_none_or(|smallest| indent.len() < smallest.len());
        if is_smaller {
            smallest_indent = Some(indent);
        }
    }
    smallest_indent.unwrap_or("    ").to_owned()
}

fn test_members(unit: &Unit, indent: &str) -> String {
    let mut mocks = String::new();
    for name in &unit.mocks {
        let _ = writeln!(mocks, "{indent}{indent}{indent}self.{name} = None");
    }
    let table = super::super::ValueSource::case_table(&unit.cases, &unit.inputs).to_string();
    let cases = serde_json::Value::String(table);
    format!(
        "{indent}async def startBehaviors(self):\n\
         {indent}{indent}await self.stopBehaviors()\n\
         {indent}{indent}self.behaviors = await Behaviors.create({}, {{'partition': {}, 'signal': BehaviorSignal(), 'profiles': {}, 'properties': {}}})\n\
         {indent}async def stopBehaviors(self):\n\
         {indent}{indent}context = getattr(self, 'behaviors', None)\n\
         {indent}{indent}if context is not None:\n\
         {indent}{indent}{indent}context.dispose()\n\
         {indent}{indent}self.behaviors = None\n\
         {indent}async def POST(self):\n\
         {indent}{indent}cases = []\n\
         {indent}{indent}for name, method, arguments in _behavior_json.loads({cases}):\n\
{mocks}\
         {indent}{indent}{indent}await self.startBehaviors()\n\
         {indent}{indent}{indent}BehaviorCalls.take()\n\
         {indent}{indent}{indent}try:\n\
         {indent}{indent}{indent}{indent}await _behavior_await(getattr(self, method)(*arguments), _BehaviorExecution(self.behaviors))\n\
         {indent}{indent}{indent}{indent}cases.append({{'name': name, 'passed': True, 'called': BehaviorCalls.take()}})\n\
         {indent}{indent}{indent}except Exception as error:\n\
         {indent}{indent}{indent}{indent}cases.append({{'name': name, 'passed': False, 'error': str(error), 'called': BehaviorCalls.take()}})\n\
         {indent}{indent}{indent}finally:\n\
         {indent}{indent}{indent}{indent}await self.stopBehaviors()\n\
         {indent}{indent}return {{'unit': {}, 'cases': cases}}\n",
        unit.configuration,
        serde_json::Value::String(unit.partition.clone()),
        unit.profiles,
        unit.flat_properties(),
        serde_json::Value::String(unit.name.clone()),
    )
}
