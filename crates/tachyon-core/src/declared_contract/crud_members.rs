use super::crud_template::CrudTemplate;
use super::member_indent::MemberIndent;
use super::repository_base::RepositoryBase;

pub(crate) struct CrudMembers<'a> {
    pub(crate) language: &'a str,
    pub(crate) base: &'a RepositoryBase,
    pub(crate) entity: &'a str,
    pub(crate) owner: &'a str,
    pub(crate) receiver: &'a str,
    pub(crate) indent: &'a str,
}

impl CrudMembers<'_> {
    pub(crate) fn text(&self) -> String {
        CrudTemplate::find(self.language).map_or_else(String::new, |template| {
            MemberIndent {
                indent: self.indent,
            }
            .block(&self.filled(template))
        })
    }

    fn filled(&self, template: CrudTemplate) -> String {
        let written = [self.base.id.as_str(), template.id];
        let id = written[usize::from(self.base.id.is_empty())];
        template
            .members
            .replace("{id}", id)
            .replace("{entity}", self.entity)
            .replace("{owner}", self.owner)
            .replace("{receiver}", self.receiver)
    }
}
