use super::{BlockKind, TypeForm, TypeHeader};

pub(crate) struct OpenBlock {
    pub(crate) indent: usize,
    pub(crate) kind: BlockKind,
    pub(crate) closer: String,
    pub(crate) form: TypeForm,
    pub(crate) name: String,
    pub(crate) fields_open: bool,
    pub(crate) parameters: String,
}

impl OpenBlock {
    pub(crate) fn body(indent: usize, kind: BlockKind, closer: &str) -> Self {
        Self {
            indent,
            kind,
            closer: closer.to_owned(),
            form: TypeForm::Class,
            name: String::new(),
            fields_open: false,
            parameters: String::new(),
        }
    }

    pub(crate) fn declared_type(indent: usize, header: &TypeHeader) -> Self {
        Self {
            indent,
            kind: BlockKind::Type,
            closer: String::from(" }"),
            form: header.form,
            name: header.name.clone(),
            fields_open: false,
            parameters: header.parameters.clone(),
        }
    }
}
