use super::LuaModels;
use crate::route_types::TypeText;
use crate::yon::plan::{ParameterSource, Plan, Provider, Route, Scope};
use crate::yon::route_calls::RouteCalls;
use std::fmt::Write as _;

pub(super) struct LuaLowering<'p> {
    plan: &'p Plan,
}

impl<'p> LuaLowering<'p> {
    const TYPE_SYNTAX: &'static str = "rb";
    const NO_ROUTE: &'static str = "error(setmetatable({ message = \"No controller method answers this route.\" }, YonNoRoute), 0)";

    pub(super) fn new(plan: &'p Plan) -> Self {
        Self { plan }
    }

    pub(super) fn program(&self, authored: &str) -> Result<String, String> {
        let controller = self.provider_index(&self.plan.controller)?;
        let mut output = format!("{authored}\n{}", self.providers()?);
        for verb in RouteCalls::verbs(self.plan) {
            self.route_function(&mut output, verb);
        }
        self.error_function(&mut output);
        output.push_str("Handler = { YON_FRAMES = true }\n\n");
        for verb in RouteCalls::verbs(self.plan) {
            let _ = writeln!(
                output,
                "function Handler.{verb}(request)\n  local cache = {{}}\n  return YonGenerated.route_{verb}(YonGenerated.resolve_{controller}(cache), request{})\nend\n",
                self.error_owner_calls()
            );
        }
        Ok(output)
    }

    pub(super) fn providers(&self) -> Result<String, String> {
        let mut output = String::from("YonGenerated = { providers = {} }\n\n");
        for (index, provider) in self.plan.providers.iter().enumerate() {
            self.provider(&mut output, provider, index)?;
        }
        Ok(output)
    }

    pub(super) fn provider_index(&self, name: &str) -> Result<usize, String> {
        self.plan
            .providers
            .iter()
            .position(|provider| provider.name == name)
            .ok_or_else(|| format!("Yon provider {name} is not declared"))
    }

    fn provider(&self, output: &mut String, provider: &Provider, index: usize) -> Result<(), String> {
        let cached = provider.scope != Scope::Transient;
        let _ = writeln!(output, "function YonGenerated.resolve_{index}(cache)");
        if cached {
            let _ = writeln!(output, "  if cache[{index}] ~= nil then\n    return cache[{index}]\n  end");
        }
        let _ = writeln!(
            output,
            "  local value = setmetatable({{}}, {{ __index = {} }})",
            provider.name
        );
        for (injection, name) in provider.injections.iter().zip(&provider.dependencies) {
            let dependency = self.provider_index(name)?;
            let _ = writeln!(
                output,
                "  value.{} = YonGenerated.resolve_{dependency}(cache)",
                injection.field
            );
        }
        if cached {
            let _ = writeln!(output, "  cache[{index}] = value");
        }
        let _ = writeln!(
            output,
            "  return value\nend\nYonGenerated.providers[{}] = YonGenerated.resolve_{index}\n",
            provider.name
        );
        Ok(())
    }

    fn error_owner_calls(&self) -> String {
        self.owner_list(", YonGenerated.resolve_", "(cache)")
    }

    fn error_owner_names(&self) -> String {
        self.owner_list(", handler", "")
    }

    fn owner_list(&self, before: &str, after: &str) -> String {
        RouteCalls::error_owners(self.plan)
            .iter()
            .fold(String::new(), |mut list, (index, _)| {
                let _ = write!(list, "{before}{index}{after}");
                list
            })
    }

    fn route_function(&self, output: &mut String, verb: &str) {
        let owners = self.error_owner_names();
        let guarded = !self.plan.errors.is_empty();
        let _ = writeln!(
            output,
            "function YonGenerated.route_{verb}(controller, request{owners})\n  local values = request.arguments or {{}}\n  local route = request.route"
        );
        let indent = if guarded { "    " } else { "  " };
        if guarded {
            output.push_str("  local ok, result = pcall(function()\n");
        }
        for route in RouteCalls::routes_for(self.plan, verb) {
            let _ = writeln!(
                output,
                "{indent}if route == {} then\n{indent}  return {}\n{indent}end",
                RouteCalls::pattern(route),
                Self::call(route)
            );
        }
        let _ = writeln!(output, "{indent}{}", Self::NO_ROUTE);
        if guarded {
            let _ = writeln!(
                output,
                "  end)\n  if ok then\n    return result\n  end\n  return YonGenerated.handle_error(result, request{owners})"
            );
        }
        output.push_str("end\n\n");
    }

    fn call(route: &Route) -> String {
        let arguments = route
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                if parameter.source == ParameterSource::Request {
                    return String::from("YonRequest.of(request)");
                }
                let value = format!("values[{}]", index + 1);
                Self::read(&parameter.type_text, &value)
            })
            .collect::<Vec<_>>()
            .join(", ");
        Self::read(&route.returns, &format!("controller:{}({arguments})", route.method))
    }

    fn read(type_text: &str, value: &str) -> String {
        TypeText::shape(Self::TYPE_SYNTAX, type_text)
            .ok()
            .and_then(|shape| LuaModels::descriptor(&shape))
            .map_or_else(|| value.to_owned(), |descriptor| format!("YonModels.read({descriptor}, {value})"))
    }

    fn error_function(&self, output: &mut String) {
        if self.plan.errors.is_empty() {
            return;
        }
        let owners = self.error_owner_names();
        let _ = writeln!(
            output,
            "function YonGenerated.handle_error(failure, request{owners})\n  return YonErrors.handle(failure, {{"
        );
        for method in &self.plan.errors {
            let Some((index, _)) = RouteCalls::error_owners(self.plan)
                .into_iter()
                .find(|(_, name)| *name == method.owner)
            else {
                continue;
            };
            let request = if method.takes_request { ", YonRequest.of(request)" } else { "" };
            let _ = writeln!(
                output,
                "    {{ {}, {}, function() return handler{index}:{}(failure{request}) end }},",
                method.error_type, method.status, method.method
            );
        }
        output.push_str("  })\nend\n\n");
    }
}
