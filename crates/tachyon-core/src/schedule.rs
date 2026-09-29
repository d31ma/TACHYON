use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Schedule {
    pub(crate) every_seconds: u64,
    pub(crate) initial_seconds: u64,
}

impl Schedule {
    const MIN_SECONDS: u64 = 1;
    const MAX_SECONDS: u64 = 86_400;
    const KEYS: [&str; 2] = ["fixedDelaySeconds", "initialDelaySeconds"];
}

pub(crate) fn declared(relative: &str, source: &str) -> Result<Option<Schedule>, Failure> {
    let language = relative.rsplit_once('.').map_or("", |(_, value)| value);
    let mask = crate::lexical::code_mask(language, source);
    if let Some(error) = mask.error {
        return Err(failure(relative, 0, error));
    }
    let code = visible_code(source, &mask.code);
    let prefix = annotation_prefix(language);
    let mut found = None;
    let mut offset = 0;
    for line in code.split_inclusive('\n') {
        let at = offset + line.len() - line.trim_start().len();
        offset += line.len();
        if !is_scheduled_marker(line, prefix) {
            continue;
        }
        if found.is_some() {
            return Err(failure(
                relative,
                at,
                "A worker declares one Schedule marker; split the work into two workers instead",
            ));
        }
        found = Some(parse(relative, at, &source[at..offset])?);
    }
    Ok(found)
}

fn visible_code(source: &str, visible: &[bool]) -> String {
    let mut code = String::new();
    for (byte, is_visible) in source.bytes().zip(visible) {
        if *is_visible || matches!(byte, b'\n' | b'\r') {
            code.push(char::from(byte));
        } else {
            code.push(' ');
        }
    }
    code
}

fn annotation_prefix(language: &str) -> &'static str {
    match language {
        "rs" | "php" | "rb" => "#[",
        "cs" => "[",
        "fs" => "[<",
        _ => "@",
    }
}

fn is_scheduled_marker(line: &str, prefix: &str) -> bool {
    let Some(rest) = line.trim().strip_prefix(prefix) else {
        return false;
    };
    let name = rest
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .next()
        .unwrap_or_default();
    matches!(name, "Schedule" | "schedule")
}

fn parse(relative: &str, at: usize, line: &str) -> Result<Schedule, Failure> {
    let object = arguments_object(relative, at, line)?;
    let has_unknown_key = object
        .keys()
        .any(|key| !Schedule::KEYS.contains(&key.as_str()));
    if has_unknown_key {
        return Err(failure(
            relative,
            at,
            "Schedule accepts fixedDelaySeconds and the optional initialDelaySeconds",
        ));
    }
    let every = seconds(&object, "fixedDelaySeconds", Schedule::MIN_SECONDS);
    let Some(every_seconds) = every else {
        return Err(failure(
            relative,
            at,
            "Schedule.fixedDelaySeconds must be an integer from 1 to 86400",
        ));
    };
    if !object.contains_key("initialDelaySeconds") {
        return Ok(Schedule {
            every_seconds,
            initial_seconds: every_seconds,
        });
    }
    let Some(initial_seconds) = seconds(&object, "initialDelaySeconds", 0) else {
        return Err(failure(
            relative,
            at,
            "Schedule.initialDelaySeconds must be an integer from 0 to 86400",
        ));
    };
    Ok(Schedule {
        every_seconds,
        initial_seconds,
    })
}

fn arguments_object(relative: &str, at: usize, line: &str) -> Result<Map<String, Value>, Failure> {
    let incomplete = "Schedule requires one complete own-line argument list";
    let Some(open) = line.find('(') else {
        return Err(failure(relative, at, incomplete));
    };
    let close = line.rfind(')').filter(|close| *close > open);
    let Some(close) = close else {
        return Err(failure(relative, at, incomplete));
    };
    let parsed = crate::behavior::literal::parse(&line[open + 1..close]).ok();
    let Some(Value::Object(object)) = parsed else {
        return Err(failure(
            relative,
            at,
            "Schedule configuration must be a strict JSON object, never an expression",
        ));
    };
    Ok(object)
}

fn seconds(object: &Map<String, Value>, key: &str, minimum: u64) -> Option<u64> {
    let value = object.get(key)?.as_u64()?;
    if !(minimum..=Schedule::MAX_SECONDS).contains(&value) {
        return None;
    }
    Some(value)
}

fn failure(relative: &str, at: usize, message: impl Into<String>) -> Failure {
    let help = "A worker declares its cadence with one own-line Schedule marker or with \
             .tachyonrc.workers, never both; intervals are 1 to 86400 seconds.";
    Failure::one(diagnostic(
        1502,
        message,
        Some(String::from(help)),
        source_span(relative, at, at + 1),
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::{Schedule, declared};

    #[test]
    fn every_worker_language_declares_the_same_cadence() {
        let cases = [
            (
                "server/workers/beat.js",
                "@Schedule({\"fixedDelaySeconds\":30})\nclass BeatController { static POST() { return {}; } }",
            ),
            (
                "server/workers/beat.ts",
                "@Schedule({\"fixedDelaySeconds\":30})\nclass BeatController { static POST() { return {}; } }",
            ),
            (
                "server/workers/beat.py",
                "@schedule({\"fixedDelaySeconds\":30})\nclass BeatController:\n    pass",
            ),
            (
                "server/workers/beat.php",
                "#[Schedule({\"fixedDelaySeconds\":30})]\nclass BeatController {}",
            ),
            (
                "server/workers/beat.java",
                "@Schedule({\"fixedDelaySeconds\":30})\nclass BeatController {}",
            ),
            (
                "server/workers/beat.kt",
                "@Schedule({\"fixedDelaySeconds\":30})\nclass BeatController {}",
            ),
            (
                "server/workers/beat.cs",
                "[Schedule({\"fixedDelaySeconds\":30})]\nclass BeatController {}",
            ),
            (
                "server/workers/beat.rs",
                "#[Schedule({\"fixedDelaySeconds\":30})]\nstruct BeatController {}",
            ),
            (
                "server/workers/beat.fs",
                "[<Schedule({\"fixedDelaySeconds\":30})>]\ntype Handler() =\n    static member POST() = ()",
            ),
            (
                "server/workers/beat.dart",
                "@Schedule({\"fixedDelaySeconds\":30})\nclass BeatController {}",
            ),
            (
                "server/workers/beat.go",
                "// @Schedule({\"fixedDelaySeconds\":30})\ntype Handler struct{}",
            ),
        ];
        for (relative, source) in cases {
            let schedule = declared(relative, source)
                .unwrap_or_else(|failure| panic!("{relative}: {failure:?}"))
                .unwrap_or_else(|| panic!("{relative}: no schedule"));
            assert_eq!(
                schedule,
                Schedule {
                    every_seconds: 30,
                    initial_seconds: 30,
                },
                "{relative}"
            );
        }
    }

    #[test]
    fn lowering_erases_the_marker_without_adding_a_runtime() {
        let cases = [
            (
                "server/workers/beat.js",
                "@Schedule({\"fixedDelaySeconds\":30})\nclass Handler { static POST() { return {}; } }\n",
                "Schedule",
            ),
            (
                "server/workers/beat.py",
                "@schedule({\"fixedDelaySeconds\":30})\nclass Handler:\n    @staticmethod\n    def POST(request):\n        return {}\n",
                "scheduled",
            ),
            (
                "server/workers/beat.php",
                "#[Schedule({\"fixedDelaySeconds\":30})]\nclass Handler { public static function POST($request) { return []; } }\n",
                "Schedule",
            ),
            (
                "server/workers/beat.rs",
                "#[Schedule({\"fixedDelaySeconds\":30})]\nstruct Handler {}\nimpl Handler { fn POST(&self) -> Json { Json::null() } }\n",
                "Schedule",
            ),
            (
                "server/workers/beat.go",
                "// @Schedule({\"fixedDelaySeconds\":30})\ntype Handler struct{}\n\nfunc (Handler) POST() any { return nil }\n",
                "Schedule",
            ),
        ];
        for (relative, source, marker) in cases {
            let lowered = crate::behavior::lower(std::path::Path::new(relative), source, "yon")
                .unwrap_or_else(|failure| panic!("{relative}: {failure:?}"))
                .unwrap_or_else(|| panic!("{relative}: the marker was left in place"));
            assert!(!lowered.contains(marker), "{relative}: {lowered}");
            assert!(
                !lowered.contains("__tachyon_create_behaviors__")
                    && !lowered.contains("BehaviorContext"),
                "{relative}: {lowered}"
            );
            assert_eq!(lowered.len(), source.len(), "{relative}: source shifted");
        }
    }

    #[test]
    fn an_initial_delay_is_separate_from_the_interval() {
        let schedule = declared(
            "server/workers/beat.js",
            "@Schedule({\"fixedDelaySeconds\":60,\"initialDelaySeconds\":0})\nclass BeatController {}",
        )
        .expect("declared")
        .expect("schedule");
        assert_eq!(schedule.every_seconds, 60);
        assert_eq!(schedule.initial_seconds, 0);
    }

    #[test]
    fn unbounded_duplicate_and_commented_markers_fail_closed() {
        for source in [
            "@Schedule({\"fixedDelaySeconds\":0})\nclass BeatController {}",
            "@Schedule({\"fixedDelaySeconds\":86401})\nclass BeatController {}",
            "@Schedule({\"fixedDelaySeconds\":10,\"cron\":\"* * * * *\"})\nclass BeatController {}",
            "@Schedule({\"initialDelaySeconds\":10})\nclass BeatController {}",
            "@Schedule(10)\nclass BeatController {}",
            "@Schedule({\"fixedDelaySeconds\":10})\n@Schedule({\"fixedDelaySeconds\":20})\nclass BeatController {}",
        ] {
            assert!(
                declared("server/workers/beat.js", source).is_err(),
                "accepted {source}"
            );
        }
        assert_eq!(
            declared(
                "server/workers/beat.js",
                "// @Schedule({\"fixedDelaySeconds\":10})\nclass BeatController {}",
            )
            .expect("declared"),
            None,
            "a commented marker schedules nothing"
        );
    }
}
