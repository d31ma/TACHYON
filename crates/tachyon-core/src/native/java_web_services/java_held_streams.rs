use super::java_code::JavaCode;
use super::java_declaration::JavaDeclaration;
use super::java_member::JavaMember;
use super::java_member_kind::JavaMemberKind;
use crate::Failure;
use crate::events::{EventModule, EventStream, HeldStreams};
use std::path::Path;

pub(super) struct JavaHeldStreams<'a> {
    path: &'a Path,
    class: &'a str,
}

impl<'a> JavaHeldStreams<'a> {
    const CLASS: &'static str = "class";
    const TOO_MANY: &'static str = "A Java web service holds at most 16 @StreamRequest and @SocketRequest methods through its clients.";

    pub(super) fn new(path: &'a Path, class: &'a str) -> Self {
        Self { path, class }
    }

    pub(super) fn members(
        &self,
        module: Option<&EventModule>,
        source: &str,
    ) -> Result<Vec<JavaMember>, Failure> {
        module.map_or_else(|| Ok(Vec::new()), |module| self.held(module, source))
    }

    fn held(&self, module: &EventModule, source: &str) -> Result<Vec<JavaMember>, Failure> {
        let field_types = self.field_types(source);
        let streams = HeldStreams::held(&HeldStreams::clients(module, self.class), &field_types);
        (streams.len() <= HeldStreams::MOST_STREAMS)
            .then(|| {
                streams
                    .into_iter()
                    .map(|(client, stream)| Self::member(client, stream))
                    .collect()
            })
            .ok_or_else(|| {
                super::super::host::native_tool_failure(
                    1405,
                    &format!("{}: {}", self.path.display(), Self::TOO_MANY),
                )
            })
    }

    fn field_types(&self, source: &str) -> Vec<String> {
        let code = JavaCode::new(source);
        code.declaration(Self::CLASS, self.class)
            .and_then(|declared| code.body_after(declared))
            .map(|body| code.member_headers(&body))
            .unwrap_or_default()
            .iter()
            .filter_map(|header| JavaDeclaration::parse(&code.text(header)).field_type())
            .collect()
    }

    fn member(client: &str, stream: &EventStream) -> JavaMember {
        let description = serde_json::json!({
            "client": client,
            "path": stream.path,
            "event": stream.event,
            "socket": stream.socket,
            "models": stream.models,
        });
        JavaMember {
            name: format!("__stream_{client}_{}", stream.method),
            value_type: String::new(),
            parameters: Vec::new(),
            kind: JavaMemberKind::Stream {
                description: description.to_string(),
            },
        }
    }
}
