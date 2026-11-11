#![allow(clippy::expect_used)]

use super::{EventTimer, extract};
use std::fmt::Write as _;
use std::path::Path;

fn timers(source: &str) -> Vec<EventTimer> {
    let module = extract(Path::new("client/services/ClockService.ts"), source, "tac")
        .expect("events")
        .expect("module");
    module.types[0].timers.clone()
}

fn refusal(source: &str) -> String {
    match extract(Path::new("client/services/ClockService.ts"), source, "tac") {
        Ok(_) => panic!("the source must be refused"),
        Err(failure) => failure.to_string(),
    }
}

#[test]
fn a_duration_is_a_whole_number_and_a_unit_from_one_second_to_one_day() {
    assert_eq!(EventTimer::duration("\"60s\""), Some(60_000));
    assert_eq!(EventTimer::duration("'1500ms'"), Some(1_500));
    assert_eq!(EventTimer::duration("`5m`"), Some(300_000));
    assert_eq!(EventTimer::duration(" \"24h\" "), Some(86_400_000));
    assert_eq!(EventTimer::duration("\"999ms\""), None);
    assert_eq!(EventTimer::duration("\"25h\""), None);
    assert_eq!(EventTimer::duration("\"60\""), None);
    assert_eq!(EventTimer::duration("60s"), None);
    assert_eq!(EventTimer::duration("\"s\""), None);
    assert_eq!(EventTimer::duration("\"1.5s\""), None);
    assert_eq!(EventTimer::duration("\"99999999999999999999h\""), None);
}

#[test]
fn an_every_method_becomes_a_timer_of_its_type() {
    let found = timers(
        "export default class ClockService {\n  @Every('30s')\n  refresh() {}\n\n  @Every(\"1m\")\n  save() {}\n}\n",
    );
    assert_eq!(
        found,
        [
            EventTimer {
                method: String::from("refresh"),
                every: 30_000
            },
            EventTimer {
                method: String::from("save"),
                every: 60_000
            }
        ]
    );
}

#[test]
fn an_every_method_with_a_parameter_or_a_bad_duration_is_refused() {
    let message =
        refusal("export default class ClockService {\n  @Every('30s')\n  refresh(value) {}\n}\n");
    assert!(
        message.contains("An @Every method takes no parameters"),
        "{message}"
    );
    let message = refusal(
        "export default class ClockService {\n  @Every('half a minute')\n  refresh() {}\n}\n",
    );
    assert!(message.contains("@Every takes one duration"), "{message}");
}

#[test]
fn a_type_has_at_most_eight_every_methods() {
    let mut source = String::from("export default class ClockService {\n");
    for index in 0..=EventTimer::MOST_TIMERS {
        let _ = write!(source, "  @Every('1m')\n  refresh{index}() {{}}\n");
    }
    source.push_str("}\n");
    let message = refusal(&source);
    assert!(message.contains("at most 8 @Every methods"), "{message}");
}
