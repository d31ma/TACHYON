use super::super::testing::{Unit, polyglot_unit};
use super::{Edit, Method, Module, Type, render};
use serde_json::{Map, Value};
use std::fmt::Write as _;

pub(super) struct RubyLowering;

impl RubyLowering {
    pub(super) fn records(module: &Module) -> Result<String, String> {
        let records: Map<String, Value> = module
            .types
            .iter()
            .map(|type_declaration| {
                (
                    type_declaration.name.clone(),
                    Self::with_methods(type_declaration),
                )
            })
            .collect();
        serde_json::to_string(&records).map_err(|error| error.to_string())
    }

    fn with_methods(type_declaration: &Type) -> Value {
        let mut record = Self::type_record(type_declaration);
        for method in &type_declaration.methods {
            record["methods"][&method.name] = Value::Object(method.metadata.clone());
        }
        record
    }

    pub(super) fn lower(module: &Module) -> Result<String, String> {
        let mut edits = Vec::new();
        let mut units = String::new();
        for type_declaration in &module.types {
            for method in &type_declaration.methods {
                edits.push(Self::method_edit(module, type_declaration, method));
            }
            if let Some(unit) = polyglot_unit(type_declaration) {
                units.push_str(&Self::test_members(&unit));
            }
        }
        let encoded = Self::records(module)?;
        let suffix = format!(
            "{}\nBehaviors.register({}, {:?}, {:?})\n{units}{}",
            include_str!("ruby_runtime.rb"),
            Self::single_quoted(&encoded),
            module.environment,
            module.module_id,
            super::super::testing::unit_helper(&module.types, super::super::TestAssert::RUBY),
        );
        render(module, edits, &suffix)
    }

    fn type_record(type_declaration: &Type) -> Value {
        let mut record = serde_json::json!({
            "name": type_declaration.name,
            "configuration": type_declaration.configuration,
            "methods": {}
        });
        for (key, value) in &type_declaration.policies {
            record[key] = value.clone();
        }
        if let Some(value) = &type_declaration.cache {
            record["cache"] = value.clone();
        }
        if let Some(value) = &type_declaration.entity {
            record["entity"] = value.clone();
        }
        if let Some(value) = &type_declaration.table {
            record["table"] = Value::String(value.clone());
        }
        record
    }

    fn method_edit(module: &Module, type_declaration: &Type, method: &Method) -> Edit {
        let original = &module.source[method.body.clone()];
        let arguments = Self::argument_names(method);
        let replacement = if let Some(policy) = Self::only(method, "gateway") {
            let key = policy["key"].as_str().unwrap_or_default();
            format!(
                "; BehaviorBoundary.gateway({key:?}, [{arguments}]){}",
                Self::line_breaks(original)
            )
        } else if let Some(policy) = Self::only(method, "mapping") {
            let mapped = Self::mapping_body(policy, &arguments);
            format!("; {mapped}{}", Self::line_breaks(original))
        } else if let Some(policy) = Self::only(method, "valid") {
            let index = policy["argument"]
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .unwrap_or_default();
            let name = &method.parameters[index].name;
            format!(
                "; {name} = BehaviorValidation.apply(JSON.parse({}), {name});{original}",
                Self::single_quoted(&policy["schema"].to_string())
            )
        } else {
            let parameters = if arguments.is_empty() {
                String::new()
            } else {
                format!(" |{arguments}|")
            };
            format!(
                "; @behaviors.invoke(self, {:?}, {:?}, [{arguments}], lambda do{parameters}{original}end); ",
                type_declaration.name, method.name
            )
        };
        Edit {
            range: method.body.clone(),
            replacement,
        }
    }

    fn mapping_body(policy: &Value, arguments: &str) -> String {
        let plan = Self::single_quoted(&policy.to_string());
        match policy.get("result").and_then(Value::as_str) {
            Some(result) => Self::typed_mapping(policy, &plan, result, arguments),
            None => format!("BehaviorBoundary.map_value(JSON.parse({plan}), [{arguments}])"),
        }
    }

    fn typed_mapping(policy: &Value, plan: &str, result: &str, arguments: &str) -> String {
        if policy.get("realm").and_then(Value::as_str) == Some("tac") {
            let result = Self::single_quoted(result);
            return format!(
                "TacValues.read({result}, BehaviorBoundary.map_value(JSON.parse({plan}), [{arguments}].map {{ |value| TacValues.plain(value) }}), JSON.parse({plan})[\"models\"])"
            );
        }
        format!(
            "YonModels.read({result}, BehaviorBoundary.map_value(JSON.parse({plan}), [{arguments}].map {{ |value| YonModels.plain(value) }}), defined?(YonGenerated) && YonGenerated.respond_to?(:field_types) ? YonGenerated.field_types : {{}})"
        )
    }

    fn only<'a>(method: &'a Method, key: &str) -> Option<&'a Value> {
        if method.metadata.len() != 1 {
            return None;
        }
        method.metadata.get(key)
    }

    fn argument_names(method: &Method) -> String {
        let names: Vec<&str> = method
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect();
        names.join(", ")
    }

    fn line_breaks(original: &str) -> String {
        let breaks = original.matches('\n').count();
        let last_line = original.rsplit('\n').next().unwrap_or_default();
        let indent = if breaks == 0 {
            String::from(" ")
        } else {
            " ".repeat(last_line.len() - last_line.trim_start().len())
        };
        format!("{}{indent}", "\n".repeat(breaks))
    }

    fn single_quoted(text: &str) -> String {
        format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
    }

    fn test_members(unit: &Unit) -> String {
        let cases = Self::single_quoted(
            &super::super::ValueSource::case_table(&unit.cases, &unit.inputs).to_string(),
        );
        let mut clear_fakes = String::new();
        let mut fakes = String::new();
        for name in &unit.mocks {
            let _ = writeln!(clear_fakes, "      @{name} = nil");
            let _ = write!(fakes, ", {name:?} => @{name}");
        }
        let options = format!(
            "\"partition\" => {}, \"signal\" => BehaviorSignal.new, \"profiles\" => JSON.parse({}), \"properties\" => JSON.parse({})",
            serde_json::Value::String(unit.partition.clone()),
            Self::single_quoted(&unit.profiles.to_string()),
            Self::single_quoted(&unit.properties.to_string()),
        );
        format!(
            "\nclass {name}\n\
             \x20 def start_behaviors\n\
             \x20   stop_behaviors\n\
             \x20   @behaviors = Behaviors.create({configuration}, {{ {options} }})\n\
             \x20 end\n\n\
             \x20 def stop_behaviors\n\
             \x20   @behaviors.dispose if @behaviors\n\
             \x20   @behaviors = nil\n\
             \x20 end\n\n\
             \x20 def construct(type)\n\
             \x20   Behaviors.construct_with(type, @behaviors, {{ {fakes} }})\n\
             \x20 end\n\n\
             \x20 def POST\n\
             \x20   cases = JSON.parse({cases}).map do |name, method, input|\n\
             {clear_fakes}\
             \x20     begin\n\
             \x20       start_behaviors\n\
             \x20       BehaviorCalls.take\n\
             \x20       public_send(method, *input)\n\
             \x20       {{ \"name\" => name, \"passed\" => true, \"called\" => BehaviorCalls.take }}\n\
             \x20     rescue StandardError => error\n\
             \x20       {{ \"name\" => name, \"passed\" => false, \"error\" => error.message, \"called\" => BehaviorCalls.take }}\n\
             \x20     ensure\n\
             \x20       stop_behaviors\n\
             \x20     end\n\
             \x20   end\n\
             \x20   {{ \"unit\" => {unit_name}, \"cases\" => cases }}\n\
             \x20 end\n\
             end\n",
            name = unit.name,
            configuration = unit.configuration,
            fakes = fakes.trim_start_matches(", "),
            unit_name = serde_json::Value::String(unit.name.clone()),
        )
    }
}
