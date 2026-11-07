#![allow(clippy::expect_used)]

use std::path::Path;

struct DartHttpContextFixture;

impl DartHttpContextFixture {
    const CLIENT: &'static str = "@HTTPClient\nabstract interface class GreetingClient {\n @GetRequest(\"/greeting\")\n Future<Object?> getGreeting();\n}\n";
    const DEFAULT_SETUP: &'static str = "@Setup\nclass GreetingSetup {\n static BehaviorAdapters configure(BehaviorExecution execution) { return BehaviorAdapters(); }\n}\n";
    const FIRST_SETUP: &'static str = "@TestSetup\nclass FirstGreetingTestSetup {\n static BehaviorAdapters configure(BehaviorExecution execution) { return BehaviorAdapters(http: {'greeting': BehaviorHttpAdapter(exchange: _exchange)}); }\n static Future<Object?> _exchange(Object? request, BehaviorExecution execution) async { return 'first'; }\n}\n";
    const SECOND_SETUP: &'static str = "@Setup\nclass SecondGreetingSetup {\n static BehaviorAdapters configure(BehaviorExecution execution) { return BehaviorAdapters(http: {'greeting': BehaviorHttpAdapter(exchange: _exchange)}); }\n static Future<Object?> _exchange(Object? request, BehaviorExecution execution) async { return 'second'; }\n}\n";
    const UNIT: &'static str = r"@Test(FirstGreetingTestSetup)
class BorrowedHttpContextTest {
  Future<void> testTwoContextsRemainIndependent() async {
    final firstContext = Behaviors.create(FirstGreetingTestSetup, partition: 'greeting', signal: BehaviorSignal());
    final secondContext = Behaviors.create(SecondGreetingSetup, partition: 'greeting', signal: BehaviorSignal());
    try {
      final first = GreetingClient(firstContext);
      final second = GreetingClient(secondContext);
      Assert.isTrue(identical(firstContext, first.behaviors));
      Assert.isTrue(identical(secondContext, second.behaviors));
      Assert.equals('first', await first.getGreeting());
      Assert.equals('second', await second.getGreeting());
      Assert.equals('first', await first.getGreeting());
      firstContext.dispose();
      final error = await Assert.throws(() => first.getGreeting());
      Assert.equals('CANCELLED', (error as BehaviorError).code);
      Assert.equals('second', await second.getGreeting());
    } finally {
      firstContext.dispose();
      secondContext.dispose();
    }
  }

  Future<void> testCallerSetupIsBorrowed() async {
    final first = GreetingClient(behaviors!);
    final second = GreetingClient(behaviors!);
    Assert.isTrue(identical(behaviors, first.behaviors));
    Assert.isTrue(identical(behaviors, second.behaviors));
    Assert.equals('first', await first.getGreeting());
    Assert.equals('first', await second.getGreeting());
    stopBehaviors();
    final firstError = await Assert.throws(() => first.getGreeting());
    final secondError = await Assert.throws(() => second.getGreeting());
    Assert.equals('CANCELLED', (firstError as BehaviorError).code);
    Assert.equals('CANCELLED', (secondError as BehaviorError).code);
  }

  Future<void> testDisposedProvidedContextDoesNotFallBack() async {
    final context = Behaviors.create(FirstGreetingTestSetup, partition: 'greeting', signal: BehaviorSignal());
    context.dispose();
    final client = GreetingClient(context);
    final error = await Assert.throws(() => client.getGreeting());
    Assert.equals('CANCELLED', (error as BehaviorError).code);
  }

  Future<void> testNoArgumentClientHasNoNativeDefaultTransport() async {
    final client = GreetingClient();
    try {
      final error = await Assert.throws(() => client.getGreeting());
      Assert.equals('HTTP_MISSING', (error as BehaviorError).code);
    } finally {
      client.behaviors.dispose();
    }
  }
}
";
}

#[test]
#[ignore = "requires installed Dart; runs captured custom HTTP adapters without browser transport or network"]
fn dart_http_clients_borrow_captured_contexts_and_keep_native_http_refusal() {
    let root = tempfile::tempdir().expect("Dart context project");
    for (relative, source) in [
        (
            "client/remotes/GreetingClient.dart",
            DartHttpContextFixture::CLIENT,
        ),
        (
            "client/setups/GreetingSetup.dart",
            DartHttpContextFixture::DEFAULT_SETUP,
        ),
        (
            "client/tests/FirstGreetingTestSetup.dart",
            DartHttpContextFixture::FIRST_SETUP,
        ),
        (
            "client/setups/SecondGreetingSetup.dart",
            DartHttpContextFixture::SECOND_SETUP,
        ),
        (
            "client/tests/BorrowedHttpContextTest.dart",
            DartHttpContextFixture::UNIT,
        ),
    ] {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("authored source");
    }
    let report = super::run(
        root.path(),
        Path::new("client/tests/BorrowedHttpContextTest.dart"),
    )
    .expect("supervised native Dart Test runner");
    assert!(report.failure.is_none(), "{report:?}");
    assert_eq!(report.cases.len(), 4, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
