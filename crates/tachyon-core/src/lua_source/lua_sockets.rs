use super::{LuaLowering, LuaModels};
use crate::route_types::TypeText;
use crate::yon::dynamic::SocketCalls;
use crate::yon::plan::{Plan, SocketMethod, SocketMethods};
use std::fmt::Write as _;

pub(super) struct LuaSockets<'p> {
    plan: &'p Plan,
    methods: &'p SocketMethods,
}

impl<'p> LuaSockets<'p> {
    const TYPE_SYNTAX: &'static str = "rb";
    const ENDPOINT: &'static str = "YonGenerated.SocketEndpoint = {}\nYonGenerated.SocketEndpoint.__index = YonGenerated.SocketEndpoint\n\nfunction YonGenerated.SocketEndpoint.new(target)\n  return setmetatable({ target = target }, YonGenerated.SocketEndpoint)\nend\n\n";
    const CALLBACKS: [(&'static str, [&'static str; 2]); 3] = [
        ("open", ["session", ""]),
        ("close", ["session", ""]),
        ("error", ["session", "failure"]),
    ];

    pub(super) fn new(plan: &'p Plan, methods: &'p SocketMethods) -> Self {
        Self { plan, methods }
    }

    pub(super) fn program(&self, authored: &str) -> Result<String, String> {
        let lowering = LuaLowering::new(self.plan);
        let controller = lowering.provider_index(&self.plan.controller)?;
        Ok(format!(
            "{authored}\n{}{}{}{}Handler = {{ YON_FRAMES = true }}\n\nfunction Handler.GET(request)\n  local cache = {{}}\n  YonSocket.run(YonGenerated.SocketEndpoint.new(YonGenerated.resolve_{controller}(cache)), request)\nend\n",
            lowering.providers()?,
            Self::ENDPOINT,
            self.callbacks(),
            self.message_callback()
        ))
    }

    fn callbacks(&self) -> String {
        let chosen = [&self.methods.open, &self.methods.close, &self.methods.error];
        Self::CALLBACKS.iter().zip(chosen).fold(
            String::new(),
            |mut text, ((event, values), method)| {
                let _ = write!(text, "{}", Self::callback(event, values, method.as_ref()));
                text
            },
        )
    }

    fn callback(event: &str, values: &[&str; 2], method: Option<&SocketMethod>) -> String {
        let parameters = values
            .iter()
            .filter(|value| !value.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let body = method.map_or_else(String::new, |method| {
            format!(
                "  self.target:{}({})\n",
                method.name,
                SocketCalls::arguments(method, values)
            )
        });
        format!("function YonGenerated.SocketEndpoint:{event}({parameters})\n{body}end\n\n")
    }

    fn message_callback(&self) -> String {
        format!(
            "function YonGenerated.SocketEndpoint:message(session, value)\n  self.target:{}(session, {})\nend\n\n",
            self.methods.message,
            self.message()
        )
    }

    fn message(&self) -> String {
        SocketCalls::message_type(self.plan)
            .and_then(|kind| TypeText::shape(Self::TYPE_SYNTAX, kind).ok())
            .and_then(|shape| LuaModels::descriptor(&shape))
            .map_or_else(
                || String::from("value"),
                |descriptor| format!("YonModels.read({descriptor}, value)"),
            )
    }
}
