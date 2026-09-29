use super::java_enclosure::JavaEnclosure;
use super::java_member::JavaMember;
use super::java_member_kind::JavaMemberKind;
use super::java_service::JavaService;
use super::java_value_type::JavaValueType;
use std::fmt::Write as _;

pub(super) struct JavaServiceTable<'a> {
    values: &'a JavaValueType,
}

impl<'a> JavaServiceTable<'a> {
    const FILE: &'static str = "TacServiceTable.java";
    const NOTHING: &'static str = "null";

    pub(super) fn new(values: &'a JavaValueType) -> Self {
        Self { values }
    }

    pub(super) fn file(&self, services: &[JavaService]) -> (String, String) {
        (String::from(Self::FILE), self.source(services))
    }

    fn source(&self, services: &[JavaService]) -> String {
        let mut members = String::new();
        let mut required = String::new();
        let mut factories = String::new();
        for (index, service) in services.iter().enumerate() {
            let instance = Self::quoted(&service.instance);
            let _ = writeln!(members, "            case {instance} -> service{index}();");
            let _ = writeln!(
                required,
                "            case {instance} -> new String[] {{{}}};",
                Self::names(&service.required)
            );
            factories.push_str(&self.factory(index, service));
        }
        format!(
            "final class TacServiceTable {{\n    private TacServiceTable() {{\n    }}\n\n    \
             static java.util.Map<String, Object> members(String name) {{\n        return switch (name) {{\n{members}            default -> null;\n        }};\n    }}\n\n    \
             static String[] required(String name) {{\n        return switch (name) {{\n{required}            default -> new String[0];\n        }};\n    }}\n{factories}}}\n"
        )
    }

    fn factory(&self, index: usize, service: &JavaService) -> String {
        let class = format!("{}.{}", JavaEnclosure::holder(index), service.class);
        let mut entries = String::new();
        for member in &service.members {
            let _ = writeln!(
                entries,
                "        members.put({}, {});",
                Self::quoted(Self::key(member)),
                self.entry(member)
            );
        }
        format!(
            "\n    private static java.util.Map<String, Object> service{index}() {{\n        {class} service = new {class}();\n        \
             java.util.Map<String, Object> members = new java.util.LinkedHashMap<>();\n{entries}        return members;\n    }}\n"
        )
    }

    fn key(member: &JavaMember) -> &str {
        match &member.kind {
            JavaMemberKind::Listener { key, .. } => key,
            _ => &member.name,
        }
    }

    fn entry(&self, member: &JavaMember) -> String {
        match &member.kind {
            JavaMemberKind::Field { read_only } => self.field(member, *read_only),
            JavaMemberKind::Method => self.method(member),
            JavaMemberKind::Hook { destroy } => Self::hook(member, *destroy),
            JavaMemberKind::Listener { description, .. } => self.listener(member, description),
        }
    }

    fn field(&self, member: &JavaMember, read_only: bool) -> String {
        let name = &member.name;
        let reading = self
            .values
            .reading(&member.value_type, "value")
            .unwrap_or_default();
        let write = format!("value -> service.{name} = {reading}");
        let writer = [write.as_str(), Self::NOTHING][usize::from(read_only)];
        format!("new TacField(() -> service.{name}, {writer})")
    }

    fn method(&self, member: &JavaMember) -> String {
        let arguments = self.arguments(&member.parameters);
        let body = Self::call(member, &arguments);
        format!("new TacMethod(arguments -> TacCall.result(() -> {body}))")
    }

    fn listener(&self, member: &JavaMember, description: &str) -> String {
        let value = member
            .parameters
            .first()
            .and_then(|kind| self.values.reading(kind, "value"))
            .unwrap_or_default();
        let body = Self::call(member, &value);
        format!(
            "new TacListener({}, value -> TacCall.result(() -> {body}))",
            Self::quoted(description)
        )
    }

    fn call(member: &JavaMember, arguments: &str) -> String {
        let call = format!("service.{}({arguments})", member.name);
        let without_result = format!("{{ {call}; return null; }}");
        let returns_nothing = JavaValueType::is_nothing(&member.value_type);
        if returns_nothing {
            without_result
        } else {
            call
        }
    }

    fn hook(member: &JavaMember, destroy: bool) -> String {
        format!(
            "new TacHook({destroy}, () -> TacCall.result(() -> {{ service.{}(); return null; }}))",
            member.name
        )
    }

    fn arguments(&self, parameters: &[String]) -> String {
        let mut arguments = Vec::new();
        for (index, kind) in parameters.iter().enumerate() {
            let value = format!("TacValues.argument(arguments, {index})");
            arguments.push(self.values.reading(kind, &value).unwrap_or_default());
        }
        arguments.join(", ")
    }

    fn names(names: &[String]) -> String {
        let quoted: Vec<String> = names.iter().map(|name| Self::quoted(name)).collect();
        quoted.join(", ")
    }

    fn quoted(text: &str) -> String {
        serde_json::to_string(text).unwrap_or_default()
    }
}
