pub(crate) struct HookStatement;

impl HookStatement {
    const KEY: &'static str = "KEY";
    const RECORD: &'static str = " BehaviorCalls.record(\"KEY\");";
    const LINE_RECORD: &'static str = "BehaviorCalls.record(\"KEY\")";
    const KOTLIN_EXPRESSION: &'static str = " BehaviorCalls.record(\"KEY\") behaviorThen";
    const STATEMENTS: [(&'static str, &'static str); 12] = [
        ("js", " Behaviors.called(\"KEY\");"),
        ("ts", " Behaviors.called(\"KEY\");"),
        ("java", Self::RECORD),
        ("kt", Self::RECORD),
        ("swift", Self::RECORD),
        ("cs", " BehaviorCalls.Record(\"KEY\");"),
        ("php", " \\BehaviorCalls::record('KEY');"),
        ("py", Self::LINE_RECORD),
        ("rb", Self::LINE_RECORD),
        ("dart", " BehaviorCalls.record(r'KEY');"),
        ("go", " behaviorCallsRecord(\"KEY\");"),
        ("rs", " BehaviorCalls::record(\"KEY\");"),
    ];

    pub(crate) fn statement(language: &str, key: &str) -> String {
        Self::STATEMENTS
            .iter()
            .find(|(written_in, _)| *written_in == language)
            .map_or(Self::RECORD, |(_, statement)| statement)
            .replace(Self::KEY, key)
    }

    pub(crate) fn kotlin_expression(key: &str) -> String {
        Self::KOTLIN_EXPRESSION.replace(Self::KEY, key)
    }
}
