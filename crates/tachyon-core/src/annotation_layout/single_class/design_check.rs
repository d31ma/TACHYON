use super::duplication::{ProjectFiles, ProjectSource};
use crate::migrate::MigrationFinding;
use class_file::ClassFile;
use constant_rule::ConstantRule;
use dependency_graph::DependencyGraph;
use dependency_rule::DependencyRule;
use design_problem::DesignProblem;
use design_rule::DesignRule;
use name_rule::NameRule;
use project_words::ProjectWords;
use signature_rule::SignatureRule;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;
use test_rule::TestRule;
use type_rule::TypeRule;
use visibility_rule::VisibilityRule;

mod access_scan;
mod annotation_marks;
mod braced_fields;
mod breadth_search;
mod british_spelling;
mod class_file;
mod class_member;
mod condition_spans;
mod constant_rule;
mod constant_spelling;
mod declared_name;
mod declared_names;
mod dependency_graph;
mod dependency_rule;
mod design_problem;
mod design_rule;
mod design_switches;
mod english_words;
#[cfg(test)]
mod english_words_tests;
mod field_privacy;
mod layer_report;
mod line_fields;
mod literal;
mod literal_reader;
mod member_reader;
mod member_uses;
mod method_filter;
mod name_forms;
mod name_parts;
mod name_picker;
mod name_rule;
mod named_value;
mod need;
mod parameter;
mod parameter_list;
mod path_parts;
mod private_form;
mod project_words;
mod role_layer;
mod short_words;
mod signature;
mod signature_rule;
mod source_reach;
mod struct_fields;
mod test_rule;
mod token_index;
mod type_rule;
mod unused_member;
mod visibility_rule;
mod word_check;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod visibility_tests;

#[cfg(test)]
mod utility_layer_tests;

pub(crate) use design_switches::DesignSwitches;
pub(crate) use type_rule::MissingType;
pub(crate) use unused_member::UnusedMember;

pub(crate) struct DesignCheck;

impl DesignCheck {
    pub(crate) fn diagnostics<'p>(
        files: impl Iterator<Item = (&'p Path, &'p [u8])>,
        switches: &DesignSwitches,
    ) -> Vec<Diagnostic> {
        let checked = switches.any_on();
        let sources: Vec<ProjectSource> = files
            .filter(|_| checked)
            .filter_map(|(path, bytes)| ProjectSource::from_bytes(path, bytes))
            .collect();
        Self::problems(&sources, switches.words())
            .iter()
            .filter(|problem| switches.covers(problem))
            .map(DesignProblem::diagnostic)
            .collect()
    }

    pub(crate) fn migration_findings(root: &Path) -> Vec<MigrationFinding> {
        let sources = ProjectFiles::read(root);
        let words = ProjectWords::read_folder(root);
        Self::problems(&sources, &words)
            .iter()
            .map(DesignProblem::finding)
            .collect()
    }

    pub(crate) fn unused_members(root: &Path) -> Vec<UnusedMember> {
        let members = Self::every_unused_member(root);
        Self::switched_on(root, DesignRule::Visibility, members, |member| {
            member.path.as_str()
        })
    }

    pub(crate) fn every_unused_member(root: &Path) -> Vec<UnusedMember> {
        Self::read_classes(root, VisibilityRule::unused_members)
    }

    pub(crate) fn missing_types(root: &Path) -> Vec<MissingType> {
        let missing = Self::every_missing_type(root);
        Self::switched_on(root, DesignRule::Types, missing, |found| {
            found.path.as_str()
        })
    }

    pub(crate) fn every_missing_type(root: &Path) -> Vec<MissingType> {
        Self::read_classes(root, TypeRule::missing_types)
    }

    fn read_classes<T>(root: &Path, rule: fn(&[ClassFile], &[ProjectSource]) -> Vec<T>) -> Vec<T> {
        let sources = ProjectFiles::read(root);
        let classes: Vec<ClassFile> = sources.iter().filter_map(ClassFile::read).collect();
        rule(&classes, &sources)
    }

    fn switched_on<T>(
        root: &Path,
        rule: DesignRule,
        findings: Vec<T>,
        path: fn(&T) -> &str,
    ) -> Vec<T> {
        let switches = DesignSwitches::read_folder(root);
        findings
            .into_iter()
            .filter(|finding| Self::covered(switches.as_ref(), rule, path(finding)))
            .collect()
    }

    fn covered(switches: Option<&DesignSwitches>, rule: DesignRule, path: &str) -> bool {
        switches.is_some_and(|found| found.covers_path(rule, path))
    }

    fn problems(sources: &[ProjectSource], words: &ProjectWords) -> Vec<DesignProblem> {
        let classes: Vec<ClassFile> = sources.iter().filter_map(ClassFile::read).collect();
        let graph = DependencyGraph::read(&classes);
        let mut problems = DependencyRule::problems(&graph, &classes);
        problems.extend(NameRule::problems(&classes, words));
        problems.extend(SignatureRule::problems(&classes, &graph));
        problems.extend(ConstantRule::problems(&classes));
        problems.extend(VisibilityRule::problems(&classes, sources));
        problems.extend(TypeRule::problems(&classes, sources));
        problems.extend(TestRule::problems(sources));
        problems
    }
}
