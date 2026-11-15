use crate::node_driver::NodeDriver;

struct BoundPropertyDriver;

impl BoundPropertyDriver {
    const RUNTIME: &'static str = include_str!("tac-client.js");
    const STUBS: &'static str = r"
const Rendering = { loopContents: new WeakMap() }
const defined = new Map()
const waiting = new Map()
globalThis.customElements = {
  get: (name) => defined.get(name),
  whenDefined: (name) => new Promise((resolve) => waiting.set(name, resolve)),
  upgrade: (element) => { if (defined.has(element.localName)) Object.setPrototypeOf(element, defined.get(element.localName).prototype) },
}
class CountingElement {
  static writes = []
  get data() { return this._data }
  set data(value) { CountingElement.writes.push(value); this._data = value }
}
const element = (localName, namespaceURI = 'http://www.w3.org/1999/xhtml') => {
  const created = defined.has(localName) ? new (defined.get(localName))() : {}
  return Object.assign(created, { localName, namespaceURI })
}
const define = (name, type) => { defined.set(name, type); waiting.get(name)?.() }
class ReadOnlyElement {
  get values() { return [] }
}
const consoleErrors = []
console.error = (...parts) => consoleErrors.push(parts.map(String).join(' '))
";
    const TAKES: &'static str = r"
const svg = 'http://www.w3.org/2000/svg'
console.log(JSON.stringify([
  BoundProperty.takes(element('w-chart'), [1, 2]),
  BoundProperty.takes(element('w-chart'), { title: 'Load' }),
  BoundProperty.takes(element('w-chart'), 'text'),
  BoundProperty.takes(element('w-chart'), 3),
  BoundProperty.takes(element('w-chart'), true),
  BoundProperty.takes(element('w-chart'), null),
  BoundProperty.takes(element('w-chart'), undefined),
  BoundProperty.takes(element('div'), [1, 2]),
  BoundProperty.takes(element('font-face', svg), [1, 2]),
]))
";
    const WRITE: &'static str = r"
const chart = element('w-chart')
const points = [1, 2]
BoundProperty.write(chart, 'chart-data', points)
BoundProperty.write(chart, 'options', { stacked: true })
console.log(JSON.stringify({
  sameArray: chart.chartData === points,
  options: chart.options,
  recorded: [...BoundProperty.recorded(chart).keys()],
  attribute: 'chart-data' in chart,
}))
";
    const SYNC: &'static str = r"
define('w-chart', CountingElement)
const chart = element('w-chart')
chart.options = { stacked: true }
const record = (entries) => ({ properties: new Map(entries) })
BoundProperty.sync(chart, record([['data', [1, 2]], ['options', { stacked: true }]]), record([['data', [1, 2]]]))
const afterEqual = CountingElement.writes.length
BoundProperty.sync(chart, record([['data', [1, 2]]]), record([['data', [3]]]))
console.log(JSON.stringify({ afterEqual, writes: CountingElement.writes, options: chart.options ?? null }))
";
    const LATE: &'static str = r"
const late = element('w-late')
const points = [4, 5]
BoundProperty.write(late, 'data', points)
const before = Object.hasOwn(late, 'data')
define('w-late', CountingElement)
await Promise.resolve()
await Promise.resolve()
console.log(JSON.stringify({
  before,
  ownAfter: Object.hasOwn(late, 'data'),
  throughSetter: CountingElement.writes.length === 1 && CountingElement.writes[0] === points,
  value: late.data,
}))
";
    const READ_ONLY: &'static str = r"
define('w-sparkline', ReadOnlyElement)
const first = Object.assign(element('w-sparkline'), { id: 'probe' })
const second = element('w-sparkline')
const record = (entries) => ({ properties: new Map(entries) })
const thrown = []
const attempt = (work) => { try { work() } catch (error) { thrown.push(error.name) } }
attempt(() => BoundProperty.write(first, 'values', [1, 2]))
attempt(() => BoundProperty.write(second, 'values', [3]))
attempt(() => BoundProperty.sync(first, record([['values', [1, 2]]]), record([['values', [4]]])))
attempt(() => BoundProperty.sync(first, record([['values', [4]]]), record([])))
console.log(JSON.stringify({ thrown, errors: consoleErrors.length, message: consoleErrors[0] ?? '' }))
";
    const READ_ONLY_LATE: &'static str = r"
const late = element('w-gauge')
BoundProperty.write(late, 'values', [4, 5])
define('w-gauge', ReadOnlyElement)
await Promise.resolve()
await Promise.resolve()
console.log(JSON.stringify({ errors: consoleErrors.length, message: consoleErrors[0] ?? '', own: Object.hasOwn(late, 'values') }))
";

    fn run(harness: &str) -> serde_json::Value {
        let source = format!(
            "{}{}\n{}\n{}\n{}\n{harness}",
            Self::STUBS,
            Self::part("const plainLoopData = ", "\n}\n"),
            Self::part("const loopContent = ", "\n}\n"),
            Self::part("const sameLocal = ", "\n}\n"),
            Self::part("class BoundProperty {", "\n}\n"),
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
fn only_objects_and_arrays_bound_on_an_html_custom_element_become_properties() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    assert_eq!(
        BoundPropertyDriver::run(BoundPropertyDriver::TAKES),
        serde_json::json!([true, true, false, false, false, false, false, false, false])
    );
}

#[test]
fn a_bound_property_keeps_the_same_object_and_takes_the_camel_case_name() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    assert_eq!(
        BoundPropertyDriver::run(BoundPropertyDriver::WRITE),
        serde_json::json!({
            "sameArray": true,
            "options": { "stacked": true },
            "recorded": ["chartData", "options"],
            "attribute": false,
        })
    );
}

#[test]
fn an_update_skips_equal_values_writes_changed_values_and_clears_removed_bindings() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    assert_eq!(
        BoundPropertyDriver::run(BoundPropertyDriver::SYNC),
        serde_json::json!({ "afterEqual": 0, "writes": [[3]], "options": null })
    );
}

#[test]
fn a_value_given_before_the_element_is_defined_reaches_its_setter_after_the_definition() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    assert_eq!(
        BoundPropertyDriver::run(BoundPropertyDriver::LATE),
        serde_json::json!({ "before": true, "ownAfter": false, "throughSetter": true, "value": [4, 5] })
    );
}

#[test]
fn a_property_the_element_will_not_take_logs_one_error_and_never_throws() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let result = BoundPropertyDriver::run(BoundPropertyDriver::READ_ONLY);
    assert_eq!(result["thrown"], serde_json::json!([]));
    assert_eq!(result["errors"], serde_json::json!(1));
    let message = result["message"].as_str().expect("message");
    assert!(message.contains("<w-sparkline id=\"probe\">"), "{message}");
    assert!(message.contains(":values"), "{message}");
}

#[test]
fn a_property_refused_after_a_late_definition_logs_one_error() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let result = BoundPropertyDriver::run(BoundPropertyDriver::READ_ONLY_LATE);
    assert_eq!(result["errors"], serde_json::json!(1));
    assert_eq!(result["own"], serde_json::json!(false));
    let message = result["message"].as_str().expect("message");
    assert!(message.contains("<w-gauge>"), "{message}");
    assert!(message.contains(":values"), "{message}");
}
