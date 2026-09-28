use super::super::duplication::ProjectSource;
use super::class_file::ClassFile;
use super::class_member::ClassMember;
use super::design_problem::DesignProblem;
use super::design_rule::DesignRule;
use super::member_reader::MemberReader;
use super::member_uses::MemberUses;
use super::private_form::PrivateForm;
use super::source_reach::SourceReach;
use super::unused_member::UnusedMember;

pub(super) struct VisibilityRule;

impl VisibilityRule {
    const FOLDERS: [&'static str; 4] = ["services", "helpers", "controllers", "utils"];
    const KINDS: [&'static str; 2] = ["method", "field"];

    pub(super) fn problems(classes: &[ClassFile], sources: &[ProjectSource]) -> Vec<DesignProblem> {
        Self::unused(classes, sources)
            .iter()
            .map(|(class, member)| Self::problem(class, member))
            .collect()
    }

    pub(super) fn unused_members(
        classes: &[ClassFile],
        sources: &[ProjectSource],
    ) -> Vec<UnusedMember> {
        Self::unused(classes, sources)
            .iter()
            .map(|(class, member)| Self::described(class, member))
            .collect()
    }

    fn unused<'c, 'a>(
        classes: &'c [ClassFile<'a>],
        sources: &[ProjectSource],
    ) -> Vec<(&'c ClassFile<'a>, ClassMember)> {
        let reaches = SourceReach::read_all(sources);
        classes
            .iter()
            .filter(|class| Self::FOLDERS.contains(&class.folder))
            .flat_map(|class| Self::class_unused(class, &reaches))
            .collect()
    }

    fn class_unused<'c, 'a>(
        class: &'c ClassFile<'a>,
        reaches: &[SourceReach],
    ) -> Vec<(&'c ClassFile<'a>, ClassMember)> {
        let uses = MemberUses::read(class, reaches);
        MemberReader::public_members(class)
            .into_iter()
            .filter(|member| !uses.contains(&member.name))
            .map(|member| (class, member))
            .collect()
    }

    fn described(class: &ClassFile, member: &ClassMember) -> UnusedMember {
        UnusedMember {
            path: class.path.to_owned(),
            name: member.name.clone(),
            start: class.words.start(member.at),
            field: member.field,
        }
    }

    fn problem(class: &ClassFile, member: &ClassMember) -> DesignProblem {
        let kind = Self::KINDS[usize::from(member.field)];
        let message = format!(
            "The {kind} `{}` in {} is public, but nothing outside {} uses it.",
            member.name, class.name, class.name
        );
        let help = format!(
            "Make it private: {}. A member stays public only when the view reads it, another class calls it, or Tachyon calls it because of an annotation. `ty migrate fix --write` makes simple cases private for you.",
            PrivateForm::of(class.extension)
        );
        class.problem(DesignRule::Visibility, member.at, message, help)
    }
}
