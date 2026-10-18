mod handler_body;
mod handler_check;
mod native_http;
mod streaming_methods;

pub(crate) use native_http::NativeHttp;

pub(crate) use handler_check::HandlerCheck;
use std::collections::BTreeSet;
use std::ops::ControlFlow;
use std::path::Path;
pub(crate) use streaming_methods::StreamingMethods;

pub(crate) struct YonConvention;

impl YonConvention {
    pub(crate) const EXTENSIONS: [&str; 15] = [
        "js", "ts", "py", "java", "groovy", "cs", "fs", "kt", "php", "rs", "dart", "go", "rb",
        "lua", "hpp",
    ];

    pub(crate) fn is_yon_language(source: &Path) -> bool {
        Self::EXTENSIONS.contains(&Self::extension(source).as_str())
    }

    fn extension(source: &Path) -> String {
        source
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    }

    fn executable_source(source: &Path, contents: &str) -> Result<String, &'static str> {
        let mask = crate::lexical::code_mask(&Self::extension(source), contents);
        Self::check_mask(mask.error)?;
        Ok(Self::blank_non_code(contents, mask.code))
    }

    fn check_mask(error: Option<&'static str>) -> Result<(), &'static str> {
        match error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn blank_non_code(contents: &str, code: Vec<bool>) -> String {
        let mut visible = contents.as_bytes().to_vec();
        for (byte, is_code) in visible.iter_mut().zip(code) {
            *byte = Self::visible_byte(*byte, is_code);
        }
        String::from_utf8(visible).unwrap_or_default()
    }

    fn visible_byte(byte: u8, is_code: bool) -> u8 {
        if is_code || byte == b'\n' || byte == b'\r' {
            return byte;
        }
        b' '
    }

    fn identifier_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == '_'
    }

    fn is_whole_word(contents: &str, start: usize, end: usize) -> bool {
        let before = contents[..start].chars().next_back();
        let after = contents[end..].chars().next();
        [before, after].into_iter().all(Self::is_word_edge)
    }

    fn is_word_edge(character: Option<char>) -> bool {
        character.is_none_or(|character| !Self::identifier_character(character))
    }

    fn strip_leading_word<'a>(text: &'a str, words: &[&str]) -> Option<&'a str> {
        words.iter().find_map(|word| Self::after_word(text, word))
    }

    fn after_word<'a>(text: &'a str, word: &str) -> Option<&'a str> {
        let rest = text.strip_prefix(word)?;
        rest.starts_with(char::is_whitespace)
            .then(|| rest.trim_start())
    }

    fn closing_index(
        mut pairs: impl Iterator<Item = (usize, char)>,
        opening: char,
        closing: char,
    ) -> Option<usize> {
        let scan = pairs.try_fold(0_u32, |depth, (at, character)| {
            Self::bracket_step(depth, at, character, opening, closing)
        });
        scan.break_value().flatten()
    }

    fn bracket_step(
        depth: u32,
        at: usize,
        character: char,
        opening: char,
        closing: char,
    ) -> ControlFlow<Option<usize>, u32> {
        match (character == opening, character == closing, depth) {
            (true, _, _) => ControlFlow::Continue(depth + 1),
            (false, true, 0) => ControlFlow::Break(None),
            (false, true, 1) => ControlFlow::Break(Some(at)),
            (false, true, _) => ControlFlow::Continue(depth - 1),
            (false, false, _) => ControlFlow::Continue(depth),
        }
    }
}

#[must_use]
pub fn is_yon_language(source: &Path) -> bool {
    YonConvention::is_yon_language(source)
}

pub fn check_handler(source: &Path, contents: &str) -> Result<(), crate::Failure> {
    crate::yon_cpp::CppYon::either(source, contents, |source, contents| {
        crate::lua_source::LuaView::either(source, contents, HandlerCheck::check)
    })
}

#[must_use]
pub fn streaming_methods(source: &Path, contents: &str) -> BTreeSet<String> {
    StreamingMethods::find(source, contents)
}

#[cfg(test)]
mod tests {
    use super::{check_handler, streaming_methods};
    use std::path::Path;

    fn streaming(path: &str, source: &str) -> Vec<String> {
        streaming_methods(Path::new(path), source)
            .into_iter()
            .collect()
    }

    #[test]
    fn a_plain_handler_and_an_annotated_controller_are_both_checked() {
        assert!(
            check_handler(
                Path::new("server/workers/products.ts"),
                "export class Handler { static GET() { return []; } }"
            )
            .is_ok()
        );
        assert!(
            check_handler(
                Path::new("server/controllers/ProductsController.ts"),
                "@RequestRoute(\"/products\")\n@Public\nexport class ProductsController {\n  @GetRoute\n  getProducts() { return []; }\n}",
            )
            .is_ok()
        );
    }

    #[test]
    fn inferred_constructor_dependencies_preserve_stream_discovery() {
        let source = r#"
@Service()
class MoneyService { amount() { return 42; } }
@RequestRoute("/money")
@Public
@NeedsAll()
class MoneyController {
    moneyService;
    @StreamRoute
    *streamMoney() { yield { amount: this.moneyService.amount() }; }
    @PostRoute
    postMoney() { return {}; }
}
"#;
        let path = "server/controllers/MoneyController.js";
        let validation = check_handler(Path::new(path), source);
        assert!(validation.is_ok(), "{validation:?}");
        assert_eq!(streaming(path, source), ["GET"]);
    }

    #[test]
    fn a_groovy_stream_route_streams_its_get_method() {
        let source = "@RequestRoute('/ticks')\n@Public\nclass TicksController {\n    @StreamRoute\n    Iterator<Object> streamTicks() {\n        return [1, 2].iterator()\n    }\n}\n";
        let path = "server/controllers/TicksController.groovy";
        assert_eq!(streaming(path, source), ["GET"]);
    }

    #[test]
    fn yield_is_the_stream_declaration() {
        let typescript = "server/workers/events.ts";
        let methods = streaming(
            typescript,
            "export class Handler {\n static async *GET() {\n  yield { ready: true };\n }\n static POST() { return {}; }\n}",
        );
        assert_eq!(methods, ["GET"]);

        let methods = streaming(
            typescript,
            "class Helper { static *GET() { yield 1; } }\nexport class Handler { static GET() { return {}; } }",
        );
        assert!(
            methods.is_empty(),
            "helper generators are not route methods"
        );

        let methods = streaming(
            typescript,
            "export class Handler {\n static GET() { return {}; }\n static *events() { yield 1; }\n static POST() { return {}; }\n}",
        );
        assert!(
            methods.is_empty(),
            "helper methods do not make an ordinary route stream"
        );

        let methods = streaming(
            "server/workers/events.py",
            "class Handler:\n    @staticmethod\n    def GET(_request):\n        for sequence in range(64):\n            yield {\"sequence\": sequence}\n",
        );
        assert_eq!(methods, ["GET"]);
    }

    #[test]
    fn a_removed_yon_type_is_refused_with_the_native_type_of_the_language() {
        let cases = [
            (
                "server/workers/handler.java",
                "class Handler { static void GET(YonRequest request) {} }",
                "HttpExchange",
            ),
            (
                "server/workers/handler.go",
                "type Handler struct{}\n\nfunc (Handler) GET() *YonResponse { return nil }\n",
                "*http.Response from net/http",
            ),
            (
                "server/workers/handler.rs",
                "struct Handler;\nimpl Handler { fn GET() -> YonResponse { YonResponse::empty(204) } }\n",
                "http::Response<Vec<u8>>",
            ),
            (
                "middleware.cs",
                "sealed class Middleware { public static YonResponse before(YonRequest request) => null; }",
                "HttpRequestMessage",
            ),
        ];
        for (path, source, native) in cases {
            let Err(failure) = check_handler(Path::new(path), source) else {
                panic!("{path} must be refused");
            };
            let rendered = failure.to_string();
            assert!(
                rendered.contains("which Tachyon removed"),
                "{path}: {rendered}"
            );
            assert!(rendered.contains(native), "{path}: {rendered}");
        }
    }

    #[test]
    fn a_removed_yon_name_in_a_comment_or_string_is_not_refused() {
        let source = "// YonRequest is gone\nclass Handler { static Object GET() { return \"YonResponse\"; } }\n";
        assert!(check_handler(Path::new("server/workers/handler.java"), source).is_ok());
    }
}
