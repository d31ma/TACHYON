use crate::route_types::{Model, ModelField, Record, Shape, TypeText};
use crate::tac_cpp::{CppClass, CppFile, CppMember, CppMemberKind, CppSource, CppTypeText};
use std::path::Path;

pub(crate) struct CppYonModels;

impl CppYonModels {
    const FILE: &'static str = "server/models/Model.hpp";

    pub(crate) fn read(source: &str) -> Vec<(String, Model)> {
        let file = CppFile::read(Path::new(Self::FILE), source).ok();
        file.map(|file| Self::model(&file.class))
            .into_iter()
            .collect()
    }

    fn model(class: &CppClass) -> (String, Model) {
        let model =
            [Self::record(class), Model::Alias(Shape::Text)][usize::from(class.is_enum)].clone();
        (class.name.clone(), model)
    }

    fn record(class: &CppClass) -> Model {
        let fields = class
            .members
            .iter()
            .filter(|member| member.is_public_instance(CppMemberKind::Field))
            .map(Self::field);
        let mut record = Record::default();
        for (field, readable) in fields {
            record.open |= !readable;
            record.fields.extend(field);
        }
        Model::Record(record)
    }

    fn field(member: &CppMember) -> (Option<ModelField>, bool) {
        let shape = TypeText::shape(CppSource::EXTENSION, &CppTypeText::plain(&member.type_text));
        let readable = shape.is_ok();
        let field = shape.ok().map(|shape| ModelField {
            name: member.name.clone(),
            required: [member.has_initializer, shape.is_optional()] == [false, false],
            shape,
        });
        (field, readable)
    }
}
