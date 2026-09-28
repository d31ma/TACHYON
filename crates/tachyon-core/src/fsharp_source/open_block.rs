use super::{BlockKind, TypeForm};

pub(crate) struct OpenBlock {
    pub(crate) indent: usize,
    pub(crate) kind: BlockKind,
    pub(crate) closer: String,
    pub(crate) form: TypeForm,
    pub(crate) name: String,
    pub(crate) fields_open: bool,
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
        }
    }

    pub(crate) fn declared_type(indent: usize, form: TypeForm, name: &str) -> Self {
        Self {
            indent,
            kind: BlockKind::Type,
            closer: String::from(" }"),
            form,
            name: name.to_owned(),
            fields_open: false,
        }
    }
}
