use super::dart_stream_tests::DartStreams;

struct DartNamedStop;

impl DartNamedStop {
    const FILES: [(&'static str, &'static str); 3] = [
        (
            "client/helpers/GreeterHelper.dart",
            "@Helper\nclass GreeterHelper {\n  String greet() => 'hi';\n}\n",
        ),
        (
            "client/helpers/LoudGreeterHelper.dart",
            "@Helper\nclass LoudGreeterHelper implements GreeterHelper {\n  @override\n  String greet() => 'HI';\n}\n",
        ),
        (
            "client/services/WelcomeService.dart",
            "@Service\nclass WelcomeService {\n  @Needs\n  @Named(\"LoudGreeterHelper\")\n  final GreeterHelper greeter;\n  @Needs\n  final GreeterHelper plain;\n\n\
             \x20 @OnStart\n  void start() {\n    print('hook: start welcome');\n  }\n\n\
             \x20 @OnStop\n  void stop() {\n    print('hook: stop welcome ${greeter.greet()}');\n  }\n\n\
             \x20 String welcome() => greeter.greet();\n\n\
             \x20 String hello() => plain.greet();\n}\n",
        ),
    ];

    const SERVICES: [(&'static str, &'static str); 1] =
        [("WelcomeService", "client/services/WelcomeService.dart")];

    const DRIVER: &'static str = "const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: WelcomeService, __tachyon_event_plan__: plan } = await import('./WelcomeService.js')\n\
         const { default: ApplicationEvents } = await import('./application-events.js')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const welcome = new WelcomeService({})\n\
         bus.attach(welcome, plan.types[0], { applicationEventPublisher: null })\n\
         await bus.initialize(welcome)\n\
         const greeted = [await welcome.welcome(), await welcome.hello()]\n\
         const running = lines.filter((line) => line.startsWith('hook: '))\n\
         bus.dispose(welcome)\n\
         await new Promise((resolve) => setTimeout(resolve, 50))\n\
         process.stdout.write(`${JSON.stringify({ greeted, running, lines: lines.filter((line) => line.startsWith('hook: ')) })}\\n`)\n\
         process.exit(0)\n";
}

#[tokio::test]
async fn a_dart_service_gets_the_named_helper_and_runs_its_stop_hook_when_disposed() {
    if !DartStreams::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("project: {failure}"));
    DartStreams::publish(
        root.path(),
        &DartNamedStop::FILES,
        &DartNamedStop::SERVICES,
        &["welcome"],
    )
    .await;
    let report = DartStreams::drive(root.path(), DartNamedStop::DRIVER);
    assert_eq!(
        report["greeted"],
        serde_json::json!(["HI", "hi"]),
        "{report}"
    );
    assert_eq!(
        report["running"],
        serde_json::json!(["hook: start welcome"]),
        "{report}"
    );
    assert_eq!(
        report["lines"],
        serde_json::json!(["hook: start welcome", "hook: stop welcome HI"]),
        "{report}"
    );
}
