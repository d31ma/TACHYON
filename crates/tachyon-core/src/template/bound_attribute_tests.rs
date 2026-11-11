use crate::node_driver::NodeDriver;

struct BoundAttributeDriver;

impl BoundAttributeDriver {
    const RUNTIME: &'static str = include_str!("tac-client.js");
    const HARNESS: &'static str = r"
const cases = [
  ['aria-expanded', true], ['aria-expanded', false],
  ['aria-current', true], ['aria-current', false], ['aria-current', 'page'],
  ['aria-checked', 'mixed'], ['aria-level', 2], ['aria-hidden', null], ['aria-busy', undefined],
  ['hidden', true], ['hidden', false], ['disabled', null], ['title', 'Menu'], ['title', 0],
]
console.log(JSON.stringify(cases.map(([name, value]) => BoundAttribute.text(name, value))))
";

    fn run() -> serde_json::Value {
        let source = format!(
            "{}\n{}\n{}",
            Self::part("const display = ", "\n}\n"),
            Self::part("class BoundAttribute {", "\n}\n"),
            Self::HARNESS
        );
        let mut command = NodeDriver::command();
        command.args(["--input-type=module", "--eval", &source]);
        let output = NodeDriver::output(&mut command).expect("node harness");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.trim()).expect("results")
    }

    fn part(start: &str, end: &str) -> &'static str {
        let first = Self::RUNTIME
            .find(start)
            .expect("start of the runtime part");
        let length = Self::RUNTIME[first..]
            .find(end)
            .expect("end of the runtime part")
            + end.len();
        &Self::RUNTIME[first..first + length]
    }
}

#[test]
fn a_bound_aria_boolean_writes_true_or_false_and_an_html_boolean_keeps_its_presence_rule() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    assert_eq!(
        BoundAttributeDriver::run(),
        serde_json::json!([
            "true", "false", "true", "false", "page", "mixed", "2", null, null, "", null, null,
            "Menu", "0"
        ])
    );
}
