use super::MigrationFix;
use std::fs;
use std::path::Path;

struct Samples;

impl Samples {
    const TYPESCRIPT: &str = "@RequestRoute(\"/ticks\")\nclass TicksController {\n  @GetRoute(\"/live\")\n  @Stream\n  async *getTicks() {\n    yield { level: 1 }\n  }\n}\n";
    const TYPESCRIPT_EXPECTED: &str = "@RequestRoute(\"/ticks\")\nclass TicksController {\n  @StreamRoute(\"/live\")\n  async *streamTicks() {\n    yield { level: 1 }\n  }\n}\n";
    const PYTHON: &str = "@request_route(\"/ticks\")\nclass TicksController:\n    @get_route\n    @stream\n    def get_ticks(self):\n        yield {\"level\": 1}\n";
    const PYTHON_EXPECTED: &str = "@request_route(\"/ticks\")\nclass TicksController:\n    @stream_route\n    def stream_ticks(self):\n        yield {\"level\": 1}\n";
    const CSHARP: &str = "[RequestRoute(\"/ticks\")]\nclass TicksController {\n    [Stream]\n    [GetRoute]\n    public async IAsyncEnumerable<object> GetTicks() {\n        yield return new { level = 1 };\n    }\n}\n";
    const CSHARP_EXPECTED: &str = "[RequestRoute(\"/ticks\")]\nclass TicksController {\n    [StreamRoute]\n    public async IAsyncEnumerable<object> StreamTicks() {\n        yield return new { level = 1 };\n    }\n}\n";
    const RUBY: &str = "#[RequestRoute(\"/ticks\")]\nclass TicksController\n  # @return [Tick]\n  #[GetRoute(\"/live\")]\n  #[Stream]\n  def get_ticks\n    yield Tick.new(level: 1)\n  end\nend\n";
    const RUBY_EXPECTED: &str = "#[RequestRoute(\"/ticks\")]\nclass TicksController\n  # @return [Tick]\n  #[StreamRoute(\"/live\")]\n  def stream_ticks\n    yield Tick.new(level: 1)\n  end\nend\n";
    const POST: &str = "@RequestRoute(\"/ticks\")\nclass TicksController {\n  @PostRoute\n  @Stream\n  async *postTicks() {\n    yield { level: 1 }\n  }\n}\n";
    const CALLED: &str = "@RequestRoute(\"/ticks\")\nclass TicksController {\n  @GetRoute\n  @Stream\n  async *getTicks() {\n    yield { level: 1 }\n  }\n  @GetRoute(\"/again\")\n  async getAgain() {\n    return this.getTicks()\n  }\n}\n";
}

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(path, source).expect("source");
}

fn converted(relative: &str, source: &str) -> String {
    let project = tempfile::tempdir().expect("project");
    write(project.path(), relative, source);
    let planned = MigrationFix::plan(project.path()).expect("plan");
    planned.write(project.path()).expect("write");
    let again = MigrationFix::plan(project.path()).expect("second plan");
    assert_eq!(again.change_count(), 0, "a second run changes {relative}");
    fs::read_to_string(project.path().join(relative)).expect("converted")
}

fn problems(relative: &str, source: &str) -> String {
    let project = tempfile::tempdir().expect("project");
    write(project.path(), relative, source);
    let planned = MigrationFix::plan(project.path()).expect("plan");
    let problems: Vec<String> = planned
        .manual_steps
        .iter()
        .map(|step| step.problem.clone())
        .collect();
    problems.join("\n")
}

#[test]
fn get_route_and_stream_become_one_stream_route() {
    let cases = [
        (
            "server/controllers/TicksController.ts",
            Samples::TYPESCRIPT,
            Samples::TYPESCRIPT_EXPECTED,
        ),
        (
            "server/controllers/TicksController.py",
            Samples::PYTHON,
            Samples::PYTHON_EXPECTED,
        ),
        (
            "server/controllers/TicksController.cs",
            Samples::CSHARP,
            Samples::CSHARP_EXPECTED,
        ),
        (
            "server/controllers/TicksController.rb",
            Samples::RUBY,
            Samples::RUBY_EXPECTED,
        ),
    ];
    for (relative, source, expected) in cases {
        assert_eq!(converted(relative, source), expected, "{relative}");
    }
}

#[test]
fn streams_the_fix_cannot_rename_become_manual_steps() {
    let post = problems("server/controllers/TicksController.ts", Samples::POST);
    assert!(post.contains("is not next to one @GetRoute"), "{post}");
    let called = problems("server/controllers/TicksController.ts", Samples::CALLED);
    assert!(called.contains("getTicks is used elsewhere"), "{called}");
}
