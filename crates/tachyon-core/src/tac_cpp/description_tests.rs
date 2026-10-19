#![allow(clippy::expect_used)]

use super::page_tests::PageProject;
use serde_json::json;

struct DescribedPage;

impl DescribedPage {
    const FILES: [(&'static str, &'static str); 2] = [
        (
            "client/services/TallyService.hpp",
            "class [[Service, Description(\"Keeps the score\")]] TallyService {\npublic:\n    int count = 0;\n    int add(int amount) { count += amount; return count; }\n};\n",
        ),
        (
            "client/services/QuietService.hpp",
            "class [[Service]] QuietService {\npublic:\n    int count = 0;\n};\n",
        ),
    ];

    const SERVICES: [(&'static str, &'static str); 2] =
        [("tally", "TallyService"), ("quiet", "QuietService")];
}

#[test]
fn a_cpp_description_reaches_the_registration_the_page_program_holds() {
    if !PageProject::has_compiler() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    PageProject::write(root.path(), &DescribedPage::FILES);
    let lines = [
        String::from("describe tally"),
        String::from("describe quiet"),
        PageProject::init("tally", 1, &json!({})).to_string(),
        PageProject::call("tally", 1, "add", &json!([3])).to_string(),
    ];
    let answers = PageProject::answered_lines(root.path(), &DescribedPage::SERVICES, &lines);
    assert_eq!(answers[0]["description"], "Keeps the score", "{answers:?}");
    assert_eq!(
        answers[1]["description"],
        serde_json::Value::Null,
        "{answers:?}"
    );
    assert_eq!(answers[0]["page"], answers[2]["value"], "{answers:?}");
    assert_eq!(answers[3]["value"], 3, "{answers:?}");
}
