use super::PythonMembers;
use serde_json::Value;
use std::path::Path;

struct Members;

impl Members {
    fn read(body: &str) -> Value {
        let source = format!("class ShopService:\n{body}");
        PythonMembers::read(Path::new("ShopService.py"), &source, "ShopService")
            .unwrap_or_else(|error| panic!("members: {error:?}"))
    }

    fn typed(body: &str) -> Vec<(String, String)> {
        Self::read(body)["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|field| {
                (
                    field["name"].as_str().unwrap_or_default().to_owned(),
                    field["type"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    }

    fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
        expected
            .iter()
            .map(|(name, kind)| ((*name).to_owned(), (*kind).to_owned()))
            .collect()
    }
}

#[test]
fn fields_whose_default_is_a_call_keep_their_declared_type() {
    let body = "    count: int = 6\n    label: str = str(5)\n    product: Product = Product(name=\"A\", price=\"1\")\n    tabs: list[Tab] = [Tab(id=\"a\", name=\"A\")]\n    scores: dict[str, list[int]] = dict(a=[1, (2)])\n\n    def total(self) -> int:\n        return self.count\n";
    assert_eq!(
        Members::typed(body),
        Members::pairs(&[
            ("count", "int"),
            ("label", "str"),
            ("product", "Product"),
            ("tabs", "list[Tab]"),
            ("scores", "dict[str, list[int]]"),
        ])
    );
    assert_eq!(Members::read(body)["fields"][2]["uninitialized"], false);
}

#[test]
fn strings_with_brackets_or_hashes_do_not_hide_fields() {
    let body = "    title: str = \"f(x) # not a comment\"  # note (here)\n    mark: str = ')'\n    after: int = len(\"]\")\n";
    assert_eq!(
        Members::typed(body),
        Members::pairs(&[("title", "str"), ("mark", "str"), ("after", "int")])
    );
}

#[test]
fn multi_line_defaults_keep_the_field_and_hide_their_inner_lines() {
    let body = "    tabs: list[Tab] = [\n        Tab(id=\"a\", name=\"A\"),\n    ]\n    size: int = (\n    3\n    )\n    names: dict[str, str] = {\n    \"key\": \"value\",\n    }\n    last: str = str(\n        5\n    )\n";
    assert_eq!(
        Members::typed(body),
        Members::pairs(&[
            ("tabs", "list[Tab]"),
            ("size", "int"),
            ("names", "dict[str, str]"),
            ("last", "str"),
        ])
    );
}

#[test]
fn an_equals_sign_inside_the_type_is_not_the_default() {
    let body = "    level: Annotated[int, Meta(default=1)] = 3\n    shown: Annotated[str, Meta(default=\"a\")]\n";
    let members = Members::read(body);
    assert_eq!(
        Members::typed(body),
        Members::pairs(&[
            ("level", "Annotated[int, Meta(default=1)]"),
            ("shown", "Annotated[str, Meta(default=\"a\")]"),
        ])
    );
    assert_eq!(members["fields"][0]["uninitialized"], false);
    assert_eq!(members["fields"][1]["uninitialized"], true);
}

#[test]
fn methods_nested_classes_and_plain_statements_are_not_fields() {
    let body = "    class Inner:\n        value: int = 1\n    if True:\n        hidden: int = 2\n    plain = {\"a\": 1}\n    handler = lambda value: value\n    async def load(self) -> str:\n        return str(1)\n";
    assert!(Members::typed(body).is_empty());
}

#[test]
fn two_fields_on_one_line_are_refused_by_name() {
    let source = "class ShopService:\n    count: int = 6; label: str = str(5)\n";
    let failure = PythonMembers::read(Path::new("ShopService.py"), source, "ShopService")
        .err()
        .unwrap_or_else(|| panic!("two statements on one line were accepted"));
    assert!(failure.to_string().contains("'count'"), "{failure}");
}
