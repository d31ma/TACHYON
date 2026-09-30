use super::context_template::ContextTemplate;
use super::member_indent::MemberIndent;

pub(crate) struct ContextMembers<'a> {
    pub(crate) language: &'a str,
    pub(crate) tac: bool,
    pub(crate) provided: bool,
    pub(crate) needs_context: bool,
    pub(crate) owner: &'a str,
    pub(crate) receiver: &'a str,
    pub(crate) setup: &'a str,
    pub(crate) partition: &'a str,
    pub(crate) indent: &'a str,
}

impl ContextMembers<'_> {
    const REALMS: [&'static str; 2] = ["yon", "tac"];
    const BARE: &'static str = "bare";
    const PROVIDED: &'static str = "provided";

    pub(crate) fn parts(&self) -> (String, String) {
        let template = ContextTemplate::find(&self.keys());
        let field = self.filled(template.field);
        let members = self.filled(template.members);
        let field = Some(field)
            .filter(|field| !field.is_empty())
            .map(|field| format!(" {field}"))
            .unwrap_or_default();
        let members = Some(members)
            .filter(|members| !members.is_empty())
            .map(|members| {
                MemberIndent {
                    indent: self.indent,
                }
                .block(&members)
            })
            .unwrap_or_default();
        (field, members)
    }

    fn keys(&self) -> Vec<String> {
        let realm = Self::REALMS[usize::from(self.tac)];
        let variant = [realm, Self::BARE][usize::from(!self.needs_context)];
        let variant = [variant, Self::PROVIDED][usize::from(self.provided)];
        vec![
            format!("{}-{variant}", self.language),
            self.language.to_owned(),
        ]
    }

    fn filled(&self, template: &str) -> String {
        template
            .replace("{setup}", self.setup)
            .replace("{partition}", self.partition)
            .replace("{owner}", self.owner)
            .replace("{receiver}", self.receiver)
    }
}
