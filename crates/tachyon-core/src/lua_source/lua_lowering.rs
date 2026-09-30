use super::{LuaEvents, LuaModels};
use crate::events::EventModule;
use crate::route_types::TypeText;
use crate::yon::plan::{
    ErrorMethod, ParameterSource, Plan, Provider, Route, RouteParameter, Scope,
};
use crate::yon::route_calls::RouteCalls;
use std::fmt::Write as _;

pub(super) struct LuaLowering<'p> {
    plan: &'p Plan,
    tracked: bool,
}

impl<'p> LuaLowering<'p> {
    const TYPE_SYNTAX: &'static str = "rb";
    const NO_ROUTE: &'static str = "error(setmetatable({ message = \"No controller method answers this route.\" }, YonNoRoute), 0)";
    const INDENTS: [&'static str; 2] = ["  ", "    "];
    const REQUEST: &'static str = "YonRequest.of(request)";
    const MEMBERS: [&'static str; 2] = ["{ __index = {} }", "YonEvents.members({})"];
    const STREAMED: [&'static str; 2] = ["{}", "YonStream.of(function() return {} end)"];

    pub(super) fn new(plan: &'p Plan) -> Self {
        Self {
            plan,
            tracked: false,
        }
    }

    pub(super) fn program(&self, authored: &str) -> Result<String, String> {
        let controller = self.provider_index(&self.plan.controller)?;
        Ok(format!(
            "{authored}\n{}{}{}Handler = {{ YON_FRAMES = true }}\n\n{}",
            self.providers()?,
            self.route_functions(),
            self.error_function(),
            self.handler_functions(controller)
        ))
    }

    pub(super) fn evented_program(
        plan: &'p Plan,
        authored: &str,
        events: &EventModule,
    ) -> Result<String, String> {
        let lowering = Self {
            plan,
            tracked: true,
        };
        let controller = lowering.provider_index(&plan.controller)?;
        let owners = lowering.owner_list(", YonGenerated.resolve_", "(cache)");
        Ok(format!(
            "{authored}\n{}\n{}{}{}Handler = {{ YON_FRAMES = true }}\n\n{}",
            LuaEvents::RUNTIME,
            lowering.providers()?,
            lowering.route_functions(),
            lowering.error_function(),
            LuaEvents::new(plan, events).functions(&owners, controller)?
        ))
    }

    pub(super) fn providers(&self) -> Result<String, String> {
        Ok(format!(
            "YonGenerated = {{ providers = {{}} }}\n\n{}",
            self.provider_texts()?
        ))
    }

    pub(super) fn provider_index(&self, name: &str) -> Result<usize, String> {
        self.index_of(name)
            .ok_or_else(|| format!("Yon provider {name} is not declared"))
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.plan
            .providers
            .iter()
            .position(|provider| provider.name == name)
    }

    fn provider_texts(&self) -> Result<String, String> {
        self.plan
            .providers
            .iter()
            .enumerate()
            .map(|(index, provider)| self.provider(provider, index))
            .collect()
    }

    fn provider(&self, provider: &Provider, index: usize) -> Result<String, String> {
        let cached = provider.scope != Scope::Transient;
        let name = &provider.name;
        Ok(format!(
            "function YonGenerated.resolve_{index}(cache)\n{}  local value = setmetatable({{}}, {})\n{}{}{}  return value\nend\nYonGenerated.providers[{name}] = YonGenerated.resolve_{index}\n\n",
            Self::when(
                cached,
                &format!("  if cache[{index}] ~= nil then\n    return cache[{index}]\n  end\n")
            ),
            Self::MEMBERS[usize::from(self.tracked)].replace("{}", name),
            self.injections(provider)?,
            Self::when(cached, &format!("  cache[{index}] = value\n")),
            Self::when(
                self.tracked,
                &format!("  YonEvents.track(cache, {index})\n")
            ),
        ))
    }

    fn when(wanted: bool, text: &str) -> String {
        [String::new(), text.to_owned()]
            .into_iter()
            .nth(usize::from(wanted))
            .unwrap_or_default()
    }

    fn injections(&self, provider: &Provider) -> Result<String, String> {
        provider
            .injections
            .iter()
            .zip(&provider.dependencies)
            .map(|(injection, name)| self.injection(&injection.field, name))
            .collect()
    }

    fn injection(&self, field: &str, dependency: &str) -> Result<String, String> {
        let index = self.provider_index(dependency)?;
        Ok(format!(
            "  value.{field} = YonGenerated.resolve_{index}(cache)\n"
        ))
    }

    fn owner_list(&self, before: &str, after: &str) -> String {
        RouteCalls::error_owners(self.plan)
            .iter()
            .fold(String::new(), |mut list, (index, _)| {
                let _ = write!(list, "{before}{index}{after}");
                list
            })
    }

    fn route_functions(&self) -> String {
        RouteCalls::verbs(self.plan)
            .iter()
            .map(|verb| self.route_function(verb))
            .collect()
    }

    fn handler_functions(&self, controller: usize) -> String {
        let owners = self.owner_list(", YonGenerated.resolve_", "(cache)");
        RouteCalls::verbs(self.plan)
            .iter()
            .fold(String::new(), |mut functions, verb| {
                let _ = write!(
                    functions,
                    "function Handler.{verb}(request)\n  local cache = {{}}\n  return YonGenerated.route_{verb}(YonGenerated.resolve_{controller}(cache), request{owners})\nend\n\n"
                );
                functions
            })
    }

    fn route_function(&self, verb: &str) -> String {
        let owners = self.owner_list(", handler", "");
        let guarded = !self.plan.errors.is_empty();
        let indent = Self::INDENTS[usize::from(guarded)];
        let checks = self.route_checks(verb, indent);
        let plain = format!("{checks}{indent}{}\n", Self::NO_ROUTE);
        let protected = format!(
            "  local ok, result = pcall(function()\n{plain}  end)\n  if ok then\n    return result\n  end\n  return YonGenerated.handle_error(result, request{owners})\n"
        );
        let body = Self::when(guarded, &protected) + &Self::when(!guarded, &plain);
        format!(
            "function YonGenerated.route_{verb}(controller, request{owners})\n  local values = request.arguments or {{}}\n  local route = request.route\n{body}end\n\n"
        )
    }

    fn route_checks(&self, verb: &str, indent: &str) -> String {
        RouteCalls::routes_for(self.plan, verb)
            .iter()
            .fold(String::new(), |mut checks, route| {
                let _ = write!(
                    checks,
                    "{indent}if route == {} then\n{indent}  return {}\n{indent}end\n",
                    RouteCalls::pattern(route),
                    Self::call(route)
                );
                checks
            })
    }

    fn call(route: &Route) -> String {
        let call = Self::read(
            &route.returns,
            &format!(
                "controller:{}({})",
                route.method,
                Self::arguments(&route.parameters)
            ),
        );
        Self::STREAMED[usize::from(route.streaming)].replace("{}", &call)
    }

    fn arguments(parameters: &[RouteParameter]) -> String {
        parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| Self::argument(index, parameter))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn argument(index: usize, parameter: &RouteParameter) -> String {
        let value = Self::read(&parameter.type_text, &format!("values[{}]", index + 1));
        [value, String::from(Self::REQUEST)]
            .into_iter()
            .nth(usize::from(parameter.source == ParameterSource::Request))
            .unwrap_or_default()
    }

    fn read(type_text: &str, value: &str) -> String {
        TypeText::shape(Self::TYPE_SYNTAX, type_text)
            .ok()
            .and_then(|shape| LuaModels::descriptor(&shape))
            .map_or_else(
                || value.to_owned(),
                |descriptor| format!("YonModels.read({descriptor}, {value})"),
            )
    }

    fn error_function(&self) -> String {
        let owners = self.owner_list(", handler", "");
        let function = format!(
            "function YonGenerated.handle_error(failure, request{owners})\n  return YonErrors.handle(failure, {{\n{}  }})\nend\n\n",
            self.candidates()
        );
        Self::when(!self.plan.errors.is_empty(), &function)
    }

    fn candidates(&self) -> String {
        self.plan
            .errors
            .iter()
            .filter_map(|method| self.candidate(method))
            .collect()
    }

    fn candidate(&self, method: &ErrorMethod) -> Option<String> {
        let request = Self::when(method.takes_request, &format!(", {}", Self::REQUEST));
        RouteCalls::error_owners(self.plan)
            .into_iter()
            .find_map(|(index, name)| {
                (name == method.owner).then(|| {
                    format!(
                        "    {{ {}, {}, function() return handler{index}:{}(failure{request}) end }},\n",
                        method.error_type, method.status, method.method
                    )
                })
            })
    }
}
