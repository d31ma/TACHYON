use crate::route_types::Model;

pub(super) struct KnownModel {
    path: String,
    pub(super) described: String,
    model: Model,
}

impl KnownModel {
    const TYPESCRIPT_ENDING: &'static str = ".ts";

    pub(super) fn existing(path: &str, model: Model) -> Self {
        Self {
            path: path.to_owned(),
            described: format!("the model in '{path}'"),
            model,
        }
    }

    pub(super) fn planned(path: &str, origin: &str, model: Model) -> Self {
        Self {
            path: path.to_owned(),
            described: format!("the one in '{origin}'"),
            model,
        }
    }

    pub(super) fn import_line(&self, realm: &str, name: &str) -> String {
        let inside = self.path.strip_prefix(realm).unwrap_or(&self.path);
        Self::typescript_import(inside, name)
    }

    fn typescript_import(inside: &str, name: &str) -> String {
        if !inside.ends_with(Self::TYPESCRIPT_ENDING) {
            return String::new();
        }
        format!("/** @import {{ {name} }} from '{inside}' */\n")
    }

    pub(super) fn is_same(&self, other: &Model) -> bool {
        Self::ordered(&self.model) == Self::ordered(other)
    }

    fn ordered(model: &Model) -> Model {
        let mut ordered = model.clone();
        if let Model::Record(record) = &mut ordered {
            record
                .fields
                .sort_by(|left, right| left.name.cmp(&right.name));
        }
        ordered
    }
}
