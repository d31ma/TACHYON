use super::*;

struct CsharpLayout;

impl CsharpLayout {
    const PATH: &'static str = "client/services/CounterService.cs";
    const SOURCE: &'static str = "[Service]\npublic sealed class CounterService\n{\n    public int Count = 0;\n    public void Increment() => Count += 1;\n    public System.Threading.Tasks.Task<string> AskServer()\n    {\n        return System.Threading.Tasks.Task.FromResult(\"answer\");\n    }\n    private string Hidden()\n    {\n        return \"private\";\n    }\n}\n";

    fn report(calls: RecordedCalls) -> CoverageReport {
        super::tests::report_with(&[(Self::PATH, Self::SOURCE)], calls)
    }

    fn hooked(source: &str) -> String {
        CallHooks::insert(std::path::Path::new(Self::PATH), source)
            .unwrap_or_else(|failure| panic!("CSharp coverage hooks: {failure}"))
    }
}

#[test]
fn csharp_arrow_and_next_line_body_methods_are_case_targets() {
    let report = CsharpLayout::report(RecordedCalls::default());
    assert_eq!(report.total(), 2, "{report:?}");
    assert_eq!(
        super::tests::uncovered_methods(&report),
        vec!["Increment", "AskServer"]
    );
}

#[test]
fn csharp_void_arrow_records_before_its_own_expression() {
    let output = CsharpLayout::hooked(CsharpLayout::SOURCE);
    assert!(
        output.contains(
            "Increment() { BehaviorCalls.Record(\"CounterService.Increment\"); Count += 1; }"
        ),
        "{output}"
    );
    assert!(
        output.contains("AskServer()\n    { BehaviorCalls.Record(\"CounterService.AskServer\");"),
        "{output}"
    );
    assert!(
        output.contains("private string Hidden()\n    {\n"),
        "{output}"
    );
}

#[test]
fn csharp_value_arrow_keeps_its_return() {
    let output = CsharpLayout::hooked(
        "[Service]\npublic sealed class CounterService {\n    public int Read() => 1;\n}\n",
    );
    assert!(
        output.contains("Read() { BehaviorCalls.Record(\"CounterService.Read\"); return 1; }"),
        "{output}"
    );
}

#[test]
fn csharp_throw_arrow_remains_a_throw_statement() {
    let output = CsharpLayout::hooked(
        "[Service]\npublic sealed class CounterService {\n    public int Read() => throw new System.InvalidOperationException();\n}\n",
    );
    assert!(output.contains("Read() { BehaviorCalls.Record(\"CounterService.Read\"); throw new System.InvalidOperationException(); }"), "{output}");
}

#[test]
fn csharp_another_called_method_does_not_cover_a_next_line_body() {
    let mut calls = RecordedCalls::default();
    calls.add(
        "client",
        "CounterServiceTest.testIncrement",
        &[String::from("CounterService.Increment")],
    );
    let report = CsharpLayout::report(calls);
    assert_eq!(report.covered(), 1, "{report:?}");
    assert_eq!(super::tests::uncovered_methods(&report), vec!["AskServer"]);
}

#[test]
fn csharp_static_setup_callbacks_are_not_instance_case_targets() {
    let report = super::tests::report_with(
        &[(
            "client/setups/GreetingSetup.cs",
            "[Setup]\npublic sealed class GreetingSetup\n{\n    public static BehaviorAdapters configure(BehaviorExecution execution)\n    {\n        return new BehaviorAdapters();\n    }\n}\n",
        )],
        RecordedCalls::default(),
    );
    assert_eq!(report.total(), 0, "{report:?}");
}

#[test]
fn csharp_generated_bodyless_client_has_no_case_target() {
    let report = super::tests::report_with(
        &[(
            "client/remotes/GreetingClient.cs",
            "[HTTPClient({\"path\":\"/api/greeting\"})]\npublic interface GreetingClient\n{\n    [GetRequest]\n    System.Threading.Tasks.Task<string> getGreeting();\n}\n",
        )],
        RecordedCalls::default(),
    );
    assert_eq!(report.total(), 0, "{report:?}");
}
