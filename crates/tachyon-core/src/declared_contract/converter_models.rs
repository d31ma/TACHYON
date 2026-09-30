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
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if extension == "fs" {
            return Self::fsharp(source, models, realm);
        }
        let Some(form) = ContractForm::of(extension) else {
            return Ok(None);
        };
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
        if !code
            .lines()
            .any(|line| line.trim_start().starts_with("[<Converter"))
        {
            return Ok(None);
        }
        let rewrite = Self {
            form: ContractForm::of("cs").ok_or("C# contract spelling unavailable")?,
            source,
            code,
            models,
            realm,
        };
        let mut edits = Vec::new();
        let mut offset = 0;
        let mut marked = false;
        for line in source.split_inclusive('\n') {
            let trimmed = line.trim();
            if trimmed.starts_with("[<Convert(") || trimmed == "[<Convert>]" {
                if let Some(edit) =
                    rewrite.arguments_edit(offset..offset + line.trim_end().len())?
                {
                    edits.push(edit);
                }
                marked = true;
            } else if rewrite.code[offset..offset + line.len()]
                .trim_start()
                .starts_with("abstract ")
            {
                if !marked {
                    let indent: String = line
                        .chars()
                        .take_while(|character| character.is_whitespace() && *character != '\n')
                        .collect();
                    edits.push((
                        offset..offset,
                        format!(
                            "{indent}[<Convert{}>]\n",
                            rewrite.written_arguments(Map::new())
                        ),
                    ));
                }
                marked = false;
            }
            offset += line.len();
        }
        Ok((!edits.is_empty()).then(|| Self::apply(source, edits)))
    }

    fn edits(&self) -> Result<Vec<SourceEdit>, String> {
        let mut edits = Vec::new();
        for declaration in ContractDeclaration::find(self.form, self.source, &self.code) {
            if declaration.role.normalized != Self::ROLE {
                continue;
            }
            let pattern = self.marker_pattern(declaration.marker_at);
            for member in declaration.members(self.form, &self.code) {
                if !member.method || member.is_constructor(&self.code) {
                    continue;
                }
                edits.extend(self.member_edit(&member, &pattern)?);
            }
        }
        Ok(edits)
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
        if let Some(line) = self.convert_line(markers_start..declaration_line) {
            return self.arguments_edit(line);
        }
        let indent: String = self.source[declaration_line..]
            .chars()
            .take_while(|character| [' ', '\t'].contains(character))
            .collect();
        let arguments = self.written_arguments(Map::new());
        let (prefix, suffix) = pattern;
        let marker = format!("{indent}{prefix}{}{arguments}{suffix}\n", Self::MARKER);
        Ok(Some((declaration_line..declaration_line, marker)))
    }

    fn convert_line(&self, range: Range<usize>) -> Option<Range<usize>> {
        let mut offset = range.start;
        for line in self.source[range].split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            if self.names_convert(line) {
                return Some(start..start + line.trim_end().len());
            }
        }
        None
    }

    fn names_convert(&self, line: &str) -> bool {
        let trimmed = line.trim();
        let uncommented = trimmed
            .strip_prefix(Self::COMMENT)
            .map_or(trimmed, str::trim_start);
        let Some(rest) = uncommented.strip_prefix(self.form.marker_prefix()) else {
            return false;
        };
        let name: String = rest
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        HttpClientSource::normalized(&name) == Self::MARKER.to_ascii_lowercase()
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
        let written = match (open, close) {
            (Some(open), Some(close)) => text[open + 1..close].trim(),
            _ => "",
        };
        let options = if written.is_empty() {
            Map::new()
        } else {
            match crate::behavior::literal::parse(written) {
                Ok(Value::Object(options)) => options,
                _ => return Ok(None),
            }
        };
        if options.contains_key(Self::MODELS) || options.contains_key("realm") {
            return Err(format!(
                "{} has no \"{}\" key: Tachyon reads the models from the models folder",
                Self::MARKER,
                Self::MODELS
            ));
        }
        let replaced = match (open, close) {
            (Some(open), Some(close)) => line.start + open..line.start + close + 1,
            _ => line.start + name_end..line.start + name_end,
        };
        Ok(Some((replaced, self.written_arguments(options))))
    }

    fn written_arguments(&self, mut options: Map<String, Value>) -> String {
        if !self.models.is_empty() {
            options.insert(Self::MODELS.to_owned(), Value::Object(self.models.clone()));
            options.insert("realm".to_owned(), Value::String(self.realm.to_owned()));
        }
        if options.is_empty() {
            return String::new();
        }
        format!("({})", Value::Object(options))
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
    fn ruby_tac_typed_models_fail_closed_without_a_native_reader() {
        let models = json!({"Product":{"name":"string"},"Summary":{"name":"string"}});
        let source = "#[Converter]\nclass ProductConverter\n  # @param product [Product]\n  # @return [Summary]\n  def map(product)\n  end\nend\n";
        let path = Path::new("client/converters/ProductConverter.rb");
        let rewritten =
            ConverterModels::written(path, source, models.as_object().expect("models"), "tac")
                .expect("rewrite")
                .expect("metadata");
        let failure =
            crate::behavior::lower(path, &rewritten, "tac").expect_err("missing native reader");
        assert!(
            failure.to_string().contains("native model reader"),
            "{failure}"
        );
    }
}
