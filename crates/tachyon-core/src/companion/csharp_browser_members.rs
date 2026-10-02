use super::{Member, csharp_arguments, csharp_entry, lower_first};

pub(super) struct CsharpBrowserMembers;

impl CsharpBrowserMembers {
    pub(super) fn entry(member: &Member) -> String {
        Self::declared_entry(member)
    }

    fn declared_entry(member: &Member) -> String {
        match (member.method, member.declared_type.as_deref()) {
            (true, Some("Task" | "Task<BehaviorJSON>")) => Self::task_entry(member),
            _ => csharp_entry(member),
        }
    }

    fn task_entry(member: &Member) -> String {
        let key = lower_first(&member.name);
        let arguments = csharp_arguments(member.parameters.as_deref().unwrap_or_default());
        let call = format!("Instance.{}({arguments})", member.name);
        let result = Self::result(member.declared_type.as_deref(), &call);
        format!("        [\"{key}\"] = TacMember.TaskMethod(async arguments => {{ {result} }}),\n",)
    }

    fn result(kind: Option<&str>, call: &str) -> String {
        match kind {
            Some("Task<BehaviorJSON>") => format!("return Tac.PlainJson(await {call});"),
            _ => format!("await {call}; return null;"),
        }
    }
}
