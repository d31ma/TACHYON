use super::{Page, write};
use std::path::Path;

struct NamedStopPage;

impl NamedStopPage {
    const HELPERS: [(&'static str, &'static str); 2] = [
        (
            "client/helpers/GreeterHelper.rb",
            "#[Helper]\nclass GreeterHelper\n  def greet\n    \"hi\"\n  end\nend\n",
        ),
        (
            "client/helpers/LoudGreeterHelper.rb",
            "#[Helper]\nclass LoudGreeterHelper\n  def greet\n    \"HI\"\n  end\nend\n",
        ),
    ];

    const WELCOME: &'static str = "#[Service]\nclass WelcomeService\n  #[Needs]\n  #[Named(\"LoudGreeterHelper\")]\n  # @return [GreeterHelper]\n  attr_reader :greeter\n\n  \
         #[OnStart]\n  def start\n    puts \"hook: start welcome\"\n  end\n\n  \
         #[OnStop]\n  def stop\n    puts \"hook: stop welcome #{greeter.greet}\"\n  end\n\n  \
         # @return [String]\n  def welcome\n    greeter.greet\n  end\nend\n";

    const HELLO: &'static str = "#[Service]\nclass HelloService\n  #[Needs]\n  # @return [GreeterHelper]\n  attr_reader :greeter\n\n  \
         # @return [String]\n  def hello\n    greeter.greet\n  end\nend\n";

    const DRIVER: &'static str = "const lines = []\n\
         for (const level of ['log', 'info', 'warn', 'error']) console[level] = (...values) => lines.push(values.join(' '))\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const open = async (name) => {\n\
         \x20 const module = await import(`./${name}Service.js`)\n\
         \x20 const owner = new module.default({})\n\
         \x20 bus.attach(owner, module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owner)\n\
         \x20 return owner\n\
         }\n\
         const welcome = await open('Welcome'), hello = await open('Hello')\n\
         const greeted = [welcome.welcome(), hello.hello()]\n\
         const running = lines.filter((line) => line.startsWith('hook: '))\n\
         welcome.__tachyonDispose()\n\
         hello.__tachyonDispose()\n\
         process.stdout.write(`${JSON.stringify({ greeted, running, lines: lines.filter((line) => line.startsWith('hook: ')) })}\\n`)\n\
         process.exit(0)\n";

    fn write_page(root: &Path) {
        for (relative, text) in Self::HELPERS {
            write(root, relative, text);
        }
        write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }
}

#[tokio::test]
async fn a_ruby_service_gets_the_named_helper_and_runs_its_stop_hook_when_disposed() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    NamedStopPage::write_page(root.path());
    Page::publish(
        root.path(),
        &[
            ("welcome", "WelcomeService", NamedStopPage::WELCOME),
            ("hello", "HelloService", NamedStopPage::HELLO),
        ],
    )
    .await;
    let report = Page::run(root.path(), NamedStopPage::DRIVER);
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
