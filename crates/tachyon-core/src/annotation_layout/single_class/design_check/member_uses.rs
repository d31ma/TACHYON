use super::class_file::ClassFile;
use super::name_forms::NameForms;
use super::source_reach::SourceReach;
use std::collections::BTreeSet;

pub(super) struct MemberUses {
    names: BTreeSet<String>,
    handed_over: bool,
}

impl MemberUses {
    const IGNORED_MARKS: [char; 3] = ['_', '$', '#'];

    pub(super) fn read(class: &ClassFile, reaches: &[SourceReach], delegates: &[&str]) -> Self {
        let forms = Self::forms(class.name);
        let reached: Vec<String> = delegates
            .iter()
            .flat_map(|delegate| Self::forms(delegate))
            .chain(forms.iter().cloned())
            .collect();
        let beside: Vec<&SourceReach> = reaches
            .iter()
            .filter(|reach| reach.beside(class.path, class.realm))
            .collect();
        Self {
            names: Self::used_names(&beside, &reached, class, reaches),
            handed_over: beside
                .iter()
                .any(|reach| reach.hands_over(&forms, class.path)),
        }
    }

    pub(super) fn contains(&self, name: &str) -> bool {
        self.handed_over | self.names.contains(&Self::plain(name))
    }

    pub(super) fn plain(name: &str) -> String {
        name.chars()
            .filter(|letter| !Self::IGNORED_MARKS.contains(letter))
            .map(|letter| letter.to_ascii_lowercase())
            .collect()
    }

    fn used_names(
        beside: &[&SourceReach],
        forms: &[String],
        class: &ClassFile,
        reaches: &[SourceReach],
    ) -> BTreeSet<String> {
        let bridge_names = reaches.iter().flat_map(|reach| {
            reach.sibling_bridge_names(class.path, class.realm, class.authored_extension)
        });
        beside
            .iter()
            .flat_map(|reach| reach.used_names(forms))
            .chain(bridge_names)
            .map(Self::plain)
            .collect()
    }

    fn forms(name: &str) -> Vec<String> {
        let mut forms = NameForms::of(name).to_vec();
        forms.push(crate::template::instance_name(name));
        forms
    }
}
