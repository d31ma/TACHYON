use super::super::cohesion::Method;
use super::braced_fields::BracedFields;
use super::class_file::ClassFile;
use super::declared_name::DeclaredName;
use super::line_fields::LineFields;
use super::method_filter::MethodFilter;
use super::parameter_list::ParameterList;
use super::signature::Signature;
use super::struct_fields::StructFields;

pub(super) struct DeclaredNames;

impl DeclaredNames {
    const CLASS: &'static str = "class";
    const METHOD: &'static str = "method";
    const PARAMETER: &'static str = "parameter";
    const FIELD: &'static str = "field";

    pub(super) fn read(class: &ClassFile) -> Vec<DeclaredName> {
        let mut names: Vec<DeclaredName> = class
            .find_word(class.name)
            .map(|at| DeclaredName {
                kind: Self::CLASS,
                at,
            })
            .into_iter()
            .collect();
        names.extend(Self::fields(class));
        names.extend(
            class
                .methods
                .iter()
                .filter(|method| MethodFilter::names_are_chosen(class, method))
                .flat_map(|method| Self::method_names(class, method)),
        );
        names
    }

    fn fields(class: &ClassFile) -> Vec<DeclaredName> {
        let mut found = BracedFields::read(class);
        found.extend(StructFields::read(class));
        found.extend(LineFields::read(class));
        found
            .into_iter()
            .map(|at| DeclaredName {
                kind: Self::FIELD,
                at,
            })
            .collect()
    }

    fn method_names(class: &ClassFile, method: &Method) -> Vec<DeclaredName> {
        let signature = Signature::of(class, method);
        let parameters = ParameterList::read(class, signature.parameters)
            .into_iter()
            .filter_map(|parameter| parameter.name_at)
            .map(|at| DeclaredName {
                kind: Self::PARAMETER,
                at,
            });
        std::iter::once(DeclaredName {
            kind: Self::METHOD,
            at: signature.name_at,
        })
        .chain(parameters)
        .collect()
    }
}
