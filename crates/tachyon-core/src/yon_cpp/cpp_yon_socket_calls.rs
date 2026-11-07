use crate::yon::plan::{Route, SocketMethod, SocketMethods};

pub(crate) struct CppYonSocketCalls;

impl CppYonSocketCalls {
    const VALUES: [&'static str; 2] = ["session", "error"];
    const NOTHING: &'static str = "";

    pub(crate) fn body(route: &Route, methods: &SocketMethods) -> String {
        let message = route
            .parameters
            .first()
            .map(|parameter| parameter.type_text.as_str())
            .unwrap_or_default();
        format!(
            "        tachyon::yon::SocketSession session(frame.text(\"request_id\"));\n        return tachyon::yon::YonSocket::run(session, invocation,\n            [&]() {{ {open} }},\n            [&](const tachyon::json::JsonNode& value) {{ controller.{method}(session, tachyon::yon::YonSocket::message<{message}>(value)); }},\n            [&]() {{ {close} }},\n            [&](const std::string& error) {{ (void)error; {error} }});\n",
            open = Self::call(methods.open.as_ref()),
            method = methods.message,
            close = Self::call(methods.close.as_ref()),
            error = Self::call(methods.error.as_ref()),
        )
    }

    fn call(method: Option<&SocketMethod>) -> String {
        method.map_or_else(
            || String::from(Self::NOTHING),
            |method| {
                let count = method.parameters.min(Self::VALUES.len());
                format!(
                    "controller.{}({});",
                    method.name,
                    Self::VALUES[..count].join(", ")
                )
            },
        )
    }
}
