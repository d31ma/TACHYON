use crate::route_types::{ModelTable, TypeText};
use crate::ruby_source::{RubyMembers, Yard};
use serde_json::{Map, Value, json};

pub(crate) struct RubyModelReaders<'a> {
    models: ModelTable,
    source: &'a str,
    code: &'a str,
}

impl<'a> RubyModelReaders<'a> {
    const EXTENSION: &'static str = "rb";
    const METHOD: &'static str = "def ";
    const WRITER: &'static str = "attr_accessor ";

    pub(crate) fn new(models: ModelTable, source: &'a str, code: &'a str) -> Self {
        Self {
            models,
            source,
            code,
        }
    }

    pub(crate) fn describe(&self, members: &RubyMembers) -> Value {
        let mut described = Map::new();
        let parameters = self.method_shapes(&members.methods, &mut described);
        let fields = self.field_shapes(&members.writable, &mut described);
        json!({ "parameters": parameters, "fields": fields, "models": described })
    }

    fn method_shapes(
        &self,
        methods: &[String],
        described: &mut Map<String, Value>,
    ) -> Map<String, Value> {
        let mut shapes = Map::new();
        for method in methods {
            let written = self.parameter_shapes(method, described);
            shapes.insert(crate::companion::camel_case(method), json!(written));
        }
        shapes
    }

    fn field_shapes(
        &self,
        fields: &[String],
        described: &mut Map<String, Value>,
    ) -> Map<String, Value> {
        let mut shapes = Map::new();
        for field in fields {
            let written = self.field_shape(field, described);
            shapes.insert(crate::companion::camel_case(field), Value::String(written));
        }
        shapes
    }

    fn parameter_shapes(&self, method: &str, described: &mut Map<String, Value>) -> Vec<String> {
        let written = format!("{}{method}", Self::METHOD);
        let at = self.declaration(&written).unwrap_or(self.code.len());
        let comments = Yard::comments_before(self.source, at);
        let after_name = self.code.get(at + written.len()..).unwrap_or_default();
        let mut shapes = Vec::new();
        for name in Self::parameter_names(after_name) {
            shapes.push(self.shape(&Yard::parameter_type(&comments, &name), described));
        }
        shapes
    }

    fn field_shape(&self, field: &str, described: &mut Map<String, Value>) -> String {
        let symbol = format!(":{field}");
        let comments = self
            .writer(&symbol)
            .map_or_else(String::new, |at| Yard::comments_before(self.source, at));
        self.shape(&Yard::return_type(&comments), described)
    }

    fn shape(&self, written: &str, described: &mut Map<String, Value>) -> String {
        TypeText::shape(Self::EXTENSION, written).map_or_else(
            |_| String::new(),
            |shape| self.models.shape_description(&shape, described),
        )
    }

    fn declaration(&self, written: &str) -> Option<usize> {
        self.code
            .match_indices(written)
            .find_map(|(at, _)| Self::ends_name(self.code, at + written.len()).then_some(at))
    }

    fn writer(&self, symbol: &str) -> Option<usize> {
        self.code
            .match_indices(Self::WRITER)
            .find_map(|(at, _)| Self::writes(&self.code[at..], symbol).then_some(at))
    }

    fn writes(text: &str, symbol: &str) -> bool {
        let line = text.lines().next().unwrap_or_default();
        let names = line.strip_prefix(Self::WRITER).unwrap_or_default();
        names.split(',').any(|name| name.trim() == symbol)
    }

    fn ends_name(code: &str, end: usize) -> bool {
        code[end..].chars().next().is_none_or(Self::ends_word)
    }

    fn ends_word(character: char) -> bool {
        matches!(character, '(' | ' ' | '\n' | ';')
    }

    fn parameter_names(after_name: &str) -> Vec<String> {
        let line = after_name.lines().next().unwrap_or_default().trim();
        let list = line.strip_prefix('(').map_or(line, Self::before_closing);
        list.split(',').filter_map(Self::parameter_name).collect()
    }

    fn before_closing(inside: &str) -> &str {
        inside.split(')').next().unwrap_or(inside)
    }

    fn parameter_name(part: &str) -> Option<String> {
        let name: String = part
            .trim()
            .chars()
            .take_while(|character| Self::is_name_character(*character))
            .collect();
        (!name.is_empty()).then_some(name)
    }

    fn is_name_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == '_'
    }
}

#[cfg(test)]
mod tests {
    use super::RubyModelReaders;
    use crate::route_types::ModelTable;
    use crate::ruby_source::RubyMembers;

    const MODELS: [(&str, &str); 2] = [
        (
            "client/models/Maker.rb",
            "# @!attribute [r] name\n#   @return [String]\nMaker = Data.define(:name)\n",
        ),
        (
            "client/models/Product.rb",
            "# @!attribute [r] name\n#   @return [String]\n# @!attribute [r] maker\n#   @return [Maker]\nProduct = Data.define(:name, :maker)\n",
        ),
    ];
    const SERVICE: &str = "#[Service]\nclass ShopService\n  # @return [Product]\n  attr_accessor :product\n  # @return [String]\n  attr_accessor :picked\n\n  # @param product [Product]\n  # @return [void]\n  def choose(product)\n    @picked = product.maker.name\n  end\n\n  # @param products [Array<Product>]\n  # @param note [String]\n  def keep(products, note)\n  end\nend\n";

    #[test]
    fn model_parameters_and_fields_carry_their_shapes_and_models() {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("root: {error}"));
        for (relative, source) in MODELS {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().unwrap_or(root.path()))
                .unwrap_or_else(|error| panic!("folder: {error}"));
            std::fs::write(path, source).unwrap_or_else(|error| panic!("model: {error}"));
        }
        let models = ModelTable::read_realm(root.path(), "client", "rb")
            .unwrap_or_else(|error| panic!("models: {error}"));
        let readers = RubyModelReaders::new(models, SERVICE, SERVICE);
        let described = readers.describe(&RubyMembers::read(SERVICE));
        assert_eq!(
            described["parameters"]["choose"],
            serde_json::json!(["Product"])
        );
        assert_eq!(
            described["parameters"]["keep"],
            serde_json::json!(["[]Product", "string"])
        );
        assert_eq!(described["fields"]["product"], "Product");
        assert_eq!(described["fields"]["picked"], "string");
        assert_eq!(described["models"]["Product"]["fields"]["maker"], "!Maker");
        assert!(described["models"].get("Maker").is_some(), "{described}");
    }
}
