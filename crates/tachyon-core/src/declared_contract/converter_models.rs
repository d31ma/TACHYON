use super::contract_declaration::ContractDeclaration;
use super::contract_form::ContractForm;
use super::contract_member::ContractMember;
use crate::behavior::HttpClientSource;
use serde_json::{Map, Value};
use std::ops::Range;
use std::path::Path;

pub(crate) struct ConverterModels<'a> {
    form: ContractForm,
    source: &'a str,
    code: String,
    models: &'a Map<String, Value>,
    realm: &'a str,
}

type SourceEdit = (Range<usize>, String);

type MarkerPattern = (String, String);

impl<'a> ConverterModels<'a> {
    const ROLE: &'static str = "converter";
    const MARKER: &'static str = "Convert";
    const MODELS: &'static str = "models";
    const COMMENT: &'static str = "//";

    pub(crate) fn written(
        path: &Path,
        source: &'a str,
        models: &'a Map<String, Value>,
        realm: &'a str,
    ) -> Result<Option<String>, String> {
        Self::for_extension(path, source, models, realm)
    }

    pub(super) fn edits_in(
        path: &Path,
        source: &'a str,
        models: &'a Map<String, Value>,
        realm: &'a str,
    ) -> Result<Vec<SourceEdit>, String> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let form = ContractForm::of(extension)
            .ok_or_else(|| "Unsupported converter contract".to_owned())?;
        let mask = crate::lexical::code_mask(form.language, source);
        let rewrite = Self {
            form,
            source,
            code: HttpClientSource::masked(source, &mask.code),
            models,
            realm,
        };
        rewrite.edits()
    }

    fn for_extension(
        path: &Path,
        source: &'a str,
        models: &'a Map<String, Value>,
        realm: &'a str,
    ) -> Result<Option<String>, String> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        match extension {
            "lua" => super::lua_converter_models::LuaConverterModels::written(
                path, source, models, realm,
            ),
            "fs" => Self::fsharp(source, models, realm),
            _ => Self::standard(extension, source, models, realm),
        }
    }

    fn standard(
        extension: &str,
        source: &'a str,
        models: &'a Map<String, Value>,
        realm: &'a str,
    ) -> Result<Option<String>, String> {
        ContractForm::of(extension)
            .map(|form| Self::rewrite(form, source, models, realm))
            .transpose()
            .map(Option::flatten)
    }

    fn rewrite(
        form: ContractForm,
        source: &'a str,
        models: &'a Map<String, Value>,
        realm: &'a str,
    ) -> Result<Option<String>, String> {
        let mask = crate::lexical::code_mask(form.language, source);
        let rewrite = Self {
            form,
            source,
            code: HttpClientSource::masked(source, &mask.code),
            models,
            realm,
        };
        let edits = rewrite.edits()?;
        Ok((!edits.is_empty()).then(|| Self::apply(source, edits)))
    }

    fn fsharp(
        source: &'a str,
        models: &'a Map<String, Value>,
        realm: &'a str,
    ) -> Result<Option<String>, String> {
        let mask = crate::lexical::code_mask("fs", source);
        let code = HttpClientSource::masked(source, &mask.code);
        let has_converter = code
            .lines()
            .any(|line| line.trim_start().starts_with("[<Converter"));
        has_converter
            .then(|| Self::fsharp_rewrite(source, models, realm, code))
            .transpose()
            .map(Option::flatten)
    }

    fn fsharp_rewrite(
        source: &'a str,
        models: &'a Map<String, Value>,
        realm: &'a str,
        code: String,
    ) -> Result<Option<String>, String> {
        let rewrite = Self {
            form: ContractForm::of("cs").ok_or("C# contract spelling unavailable")?,
            source,
            code,
            models,
            realm,
        };
        let edits = rewrite.fsharp_edits()?;
        Ok((!edits.is_empty()).then(|| Self::apply(source, edits)))
    }

    fn fsharp_edits(&self) -> Result<Vec<SourceEdit>, String> {
        let mut edits = Vec::new();
        let mut offset = 0;
        let mut marked = false;
        for line in self.source.split_inclusive('\n') {
            edits.extend(self.fsharp_line(line, offset, &mut marked)?);
            offset += line.len();
        }
        Ok(edits)
    }

    fn fsharp_line(
        &self,
        line: &str,
        offset: usize,
        marked: &mut bool,
    ) -> Result<Option<SourceEdit>, String> {
        let trimmed = line.trim();
        let convert = [trimmed.starts_with("[<Convert("), trimmed == "[<Convert>]"].contains(&true);
        let member = self.code[offset..offset + line.len()]
            .trim_start()
            .starts_with("abstract ");
        match (convert, member) {
            (true, _) => {
                *marked = true;
                self.arguments_edit(offset..offset + line.trim_end().len())
            }
            (_, true) => {
                let edit = (!*marked).then(|| self.fsharp_marker(line, offset));
                *marked = false;
                Ok(edit)
            }
            _ => Ok(None),
        }
    }

    fn fsharp_marker(&self, line: &str, offset: usize) -> SourceEdit {
        let indent: String = line
            .chars()
            .take_while(|character| [' ', '\t', '\r'].contains(character))
            .collect();
        (
            offset..offset,
            format!(
                "{indent}[<Convert{}>]\n",
                self.written_arguments(Map::new())
            ),
        )
    }

    fn edits(&self) -> Result<Vec<SourceEdit>, String> {
        let declarations = ContractDeclaration::find(self.form, self.source, &self.code);
        let converters = declarations
            .into_iter()
            .filter(|declaration| declaration.role.normalized == Self::ROLE);
        let edits: Result<Vec<_>, String> = converters
            .map(|declaration| self.declaration_edits(&declaration))
            .collect();
        Ok(edits?.into_iter().flatten().collect())
    }

    fn declaration_edits(
        &self,
        declaration: &ContractDeclaration,
    ) -> Result<Vec<SourceEdit>, String> {
        let pattern = self.marker_pattern(declaration.marker_at);
        let members = declaration.members(self.form, &self.code);
        let methods = members
            .into_iter()
            .filter(|member| self.mapping_member(member));
        let edits: Result<Vec<_>, String> = methods
            .map(|member| self.member_edit(&member, &pattern))
            .collect();
        Ok(edits?.into_iter().flatten().collect())
    }

    fn mapping_member(&self, member: &ContractMember) -> bool {
        [member.method, !member.is_constructor(&self.code)] == [true, true]
    }

    fn marker_pattern(&self, marker_at: usize) -> MarkerPattern {
        let line = self.line_at(marker_at).trim();
        let folded = line.to_ascii_lowercase();
        let name_at = folded.find(Self::ROLE).unwrap_or(0);
        let after = &line[name_at + Self::ROLE.len()..];
        let suffix = after
            .strip_prefix('(')
            .and_then(|rest| rest.rfind(')').map(|close| &rest[close + 1..]))
            .unwrap_or(after);
        (line[..name_at].to_owned(), suffix.to_owned())
    }

    fn member_edit(
        &self,
        member: &ContractMember,
        pattern: &MarkerPattern,
    ) -> Result<Option<SourceEdit>, String> {
        let declaration_line = Self::line_start(self.source, member.start);
        let markers_start = member
            .first_marker
            .map_or(declaration_line, |at| Self::line_start(self.source, at));
        self.convert_line(markers_start..declaration_line)
            .map_or_else(
                || Ok(Some(self.member_marker(declaration_line, pattern))),
                |line| self.arguments_edit(line),
            )
    }

    fn member_marker(&self, declaration_line: usize, pattern: &MarkerPattern) -> SourceEdit {
        let indent: String = self.source[declaration_line..]
            .chars()
            .take_while(|character| [' ', '\t'].contains(character))
            .collect();
        let arguments = self.written_arguments(Map::new());
        let (prefix, suffix) = pattern;
        let marker = format!("{indent}{prefix}{}{arguments}{suffix}\n", Self::MARKER);
        (declaration_line..declaration_line, marker)
    }

    fn convert_line(&self, range: Range<usize>) -> Option<Range<usize>> {
        let mut offset = range.start;
        self.source[range].split_inclusive('\n').find_map(|line| {
            let start = offset;
            offset += line.len();
            self.names_convert(line)
                .then_some(start..start + line.trim_end().len())
        })
    }

    fn names_convert(&self, line: &str) -> bool {
        let trimmed = line.trim();
        let uncommented = trimmed
            .strip_prefix(Self::COMMENT)
            .map_or(trimmed, str::trim_start);
        uncommented
            .strip_prefix(self.form.marker_prefix())
            .map(Self::marker_name)
            .is_some_and(|name| {
                HttpClientSource::normalized(&name) == Self::MARKER.to_ascii_lowercase()
            })
    }

    fn marker_name(rest: &str) -> String {
        rest.chars()
            .take_while(|character| Self::name_character(*character))
            .collect()
    }

    fn name_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == '_'
    }

    fn arguments_edit(&self, line: Range<usize>) -> Result<Option<SourceEdit>, String> {
        let text = &self.source[line.clone()];
        let name_end = text
            .to_ascii_lowercase()
            .find(&Self::MARKER.to_ascii_lowercase())
            .map_or(0, |at| at + Self::MARKER.len());
        let open = text[name_end..].starts_with('(').then_some(name_end);
        let close = open
            .and_then(|_| text.rfind(')'))
            .filter(|close| *close > name_end);
        let bounds = open.zip(close);
        let written = bounds.map_or("", |(open, close)| text[open + 1..close].trim());
        let replaced = bounds.map_or(
            line.start + name_end..line.start + name_end,
            |(open, close)| line.start + open..line.start + close + 1,
        );
        Self::marker_options(written)
            .map(|options| {
                self.qualified_arguments(options)
                    .map(|arguments| (replaced, arguments))
            })
            .transpose()
    }

    fn marker_options(written: &str) -> Option<Map<String, Value>> {
        written.is_empty().then(Map::new).or_else(|| {
            crate::behavior::literal::parse(written)
                .ok()
                .and_then(|value| value.as_object().cloned())
        })
    }

    fn qualified_arguments(&self, options: Map<String, Value>) -> Result<String, String> {
        Self::authored_keys(&options)?;
        Ok(self.written_arguments(options))
    }

    fn authored_keys(options: &Map<String, Value>) -> Result<(), String> {
        if options.contains_key(Self::MODELS) || options.contains_key("realm") {
            return Err(format!(
                "{} has no \"{}\" key: Tachyon reads the models from the models folder",
                Self::MARKER,
                Self::MODELS
            ));
        }
        Ok(())
    }

    fn written_arguments(&self, mut options: Map<String, Value>) -> String {
        self.add_models(&mut options);
        options
            .is_empty()
            .then(String::new)
            .unwrap_or_else(|| format!("({})", Value::Object(options)))
    }

    fn add_models(&self, options: &mut Map<String, Value>) {
        if !self.models.is_empty() {
            options.insert(Self::MODELS.to_owned(), Value::Object(self.models.clone()));
            options.insert("realm".to_owned(), Value::String(self.realm.to_owned()));
        }
    }

    fn line_at(&self, at: usize) -> &str {
        let start = Self::line_start(self.source, at);
        let end = self.source[start..]
            .find('\n')
            .map_or(self.source.len(), |offset| start + offset);
        &self.source[start..end]
    }

    fn line_start(text: &str, at: usize) -> usize {
        text[..at].rfind('\n').map_or(0, |offset| offset + 1)
    }

    fn apply(source: &str, mut edits: Vec<SourceEdit>) -> String {
        edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
        let mut output = source.to_owned();
        for (range, replacement) in edits {
            output.replace_range(range, &replacement);
        }
        output
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::ConverterModels;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn lua_marker_rewrite_keeps_brackets_inside_constants() {
        let models = json!({"Product":{"name":"!string"},"Summary":{"name":"!string"}});
        let source = "-- @Converter\nProductConverter = {}\n---@param product Product\n---@return Summary\n-- @Convert({\"fields\":[{\"target\":\"name\",\"constant\":\")]\"}]})\nfunction ProductConverter:map(product)\nend\n";
        let rewritten = ConverterModels::written(
            Path::new("server/converters/ProductConverter.lua"),
            source,
            models.as_object().expect("models"),
            "yon",
        )
        .expect("rewrite")
        .expect("metadata");
        assert!(rewritten.contains("\"constant\":\")]\""), "{rewritten}");
        assert!(
            rewritten
                .lines()
                .any(|line| line.starts_with("-- @Convert(")),
            "{rewritten}"
        );
    }

    #[test]
    fn model_metadata_is_inserted_in_each_supported_contract_spelling() {
        let models = json!({"Product":{"name":"!string"},"Summary":{"name":"!string"}});
        let samples = [
            (
                "fs",
                "[<Converter>]\ntype ProductConverter =\n    abstract map: product: Product -> Summary\n",
            ),
            (
                "java",
                "@Converter\ninterface ProductConverter {\nSummary map(Product product);\n}\n",
            ),
            (
                "groovy",
                "@Converter\ninterface ProductConverter {\nSummary map(Product product);\n}\n",
            ),
            (
                "kt",
                "@Converter\ninterface ProductConverter {\nfun map(product: Product): Summary\n}\n",
            ),
            (
                "cs",
                "[Converter]\ninterface ProductConverter {\nSummary map(Product product);\n}\n",
            ),
            (
                "php",
                "#[Converter]\ninterface ProductConverter {\npublic function map(Product $product): Summary;\n}\n",
            ),
            (
                "go",
                "// @Converter\ntype ProductConverter interface {\nMap(product Product) (Summary, error)\n}\n",
            ),
            (
                "rs",
                "#[Converter]\ntrait ProductConverter {\nfn map(&self, product: Product) -> BehaviorResult<Summary>;\n}\n",
            ),
            (
                "swift",
                "@Converter\nprotocol ProductConverter {\nfunc map(_ product: Product) throws -> Summary\n}\n",
            ),
            (
                "dart",
                "@Converter\nabstract interface class ProductConverter {\nSummary map(Product product);\n}\n",
            ),
            (
                "py",
                "@Converter\nclass ProductConverter(Protocol):\n    def map(self, product: Product) -> Summary: ...\n",
            ),
            (
                "lua",
                "-- @Converter\nProductConverter = {}\n---@param product Product\n---@return Summary\nfunction ProductConverter:map(product)\nend\n",
            ),
            (
                "rb",
                "#[Converter]\nclass ProductConverter\n    def map(product)\n    end\nend\n",
            ),
            (
                "js",
                "@Converter\nclass ProductConverter {\nasync map(product) {}\n}\n",
            ),
            (
                "ts",
                "@Converter\nclass ProductConverter {\nasync map(product: Product): Promise<Summary> {}\n}\n",
            ),
        ];
        for (extension, source) in samples {
            let path = format!("server/converters/ProductConverter.{extension}");
            let rewritten = ConverterModels::written(
                Path::new(&path),
                source,
                models.as_object().expect("models"),
                "yon",
            )
            .expect("rewrite")
            .expect(extension);
            assert!(rewritten.contains("Convert("), "{extension}: {rewritten}");
            assert!(rewritten.contains("\"models\""), "{extension}: {rewritten}");
            assert!(
                rewritten.contains("\"Product\""),
                "{extension}: {rewritten}"
            );
            if [
                "java", "groovy", "kt", "cs", "php", "go", "rs", "dart", "py", "ts",
            ]
            .contains(&extension)
            {
                let concrete = crate::declared_contract::DeclaredContract::concrete(
                    Path::new(&path),
                    &rewritten,
                    "yon",
                )
                .unwrap_or(rewritten);
                let output = crate::behavior::lower(Path::new(&path), &concrete, "yon")
                    .unwrap_or_else(|error| panic!("{extension}: {error}"))
                    .expect("lowered");
                assert!(output.contains("Summary"), "{extension}: {output}");
            }
        }
    }

    #[test]
    fn explicit_fields_survive_and_authored_model_metadata_is_refused() {
        let models = json!({"Product":{},"Summary":{}});
        let source = "@Converter\ninterface ProductConverter {\n@Convert({\"fields\":[{\"target\":\"name\",\"constant\":\"fixed\"}]})\nSummary map(Product product);\n}\n";
        let path = Path::new("server/converters/ProductConverter.java");
        let rewritten =
            ConverterModels::written(path, source, models.as_object().expect("models"), "yon")
                .expect("rewrite")
                .expect("metadata");
        assert!(rewritten.contains("\"constant\":\"fixed\""));
        let authored = source.replace("\"fields\":", "\"models\":{},\"fields\":");
        assert!(
            ConverterModels::written(path, &authored, models.as_object().expect("models"), "yon")
                .is_err()
        );
    }
    #[test]
    fn ruby_tac_typed_models_use_the_native_reader_with_compiler_owned_shapes() {
        let models = json!({"Product":{"name":"string"},"Summary":{"name":"string"}});
        let source = "#[Converter]\nclass ProductConverter\n  # @param product [Product]\n  # @return [Summary]\n  def map(product)\n  end\nend\n";
        let path = Path::new("client/converters/ProductConverter.rb");
        let rewritten =
            ConverterModels::written(path, source, models.as_object().expect("models"), "tac")
                .expect("rewrite")
                .expect("metadata");
        let lowered = crate::behavior::lower(path, &rewritten, "tac")
            .expect("native model reader")
            .expect("lowered converter");
        assert!(lowered.contains("TacValues.read('Summary'"), "{lowered}");
        assert!(lowered.contains("TacValues.plain(value)"), "{lowered}");
        assert!(
            lowered.contains("\"Summary\":{\"fields\":{\"name\":\"string\"}}"),
            "{lowered}"
        );
    }
}
