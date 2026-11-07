#![allow(clippy::expect_used)]

use std::path::Path;

struct JavaStreamUnit;

impl JavaStreamUnit {
    const FILES: [(&'static str, &'static str); 7] = [
        (
            "client/models/Tick.java",
            "record Tick(String sku, int level) {}\n",
        ),
        (
            "client/models/ChatMessage.java",
            "record ChatMessage(String text) {}\n",
        ),
        (
            "client/remotes/TicksClient.java",
            "@HTTPClient(\"/ticks\")\ninterface TicksClient {\n    @StreamRequest\n    void receive(Tick tick);\n}\n",
        ),
        (
            "client/remotes/ChatClient.java",
            "@HTTPClient(\"/chat\")\ninterface ChatClient {\n    @SocketRequest\n    void receive(ChatMessage message);\n}\n",
        ),
        (
            "client/services/ChatService.java",
            "@Service\nclass ChatService {\n    @Needs\n    private final TicksClient ticks;\n    @Needs\n    private final ChatClient chat;\n    \
             String say(String text) {\n        try {\n            chat.send(new ChatMessage(text));\n            return \"sent\";\n        } catch (IllegalStateException error) {\n            return error.getMessage();\n        }\n    }\n}\n",
        ),
        (
            "client/tests/QuietTestSetup.java",
            "@TestSetup\nclass QuietTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) { return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null); }\n}\n",
        ),
        (
            "client/tests/ChatServiceTest.java",
            "@Test(QuietTestSetup.class, {\"partition\":\"tests\"})\nclass ChatServiceTest {\n    @Fake\n    TicksClient ticks = null;\n    @Fake\n    ChatClient chat = null;\n    \
             void testSayHasNoPageToSendOn() {\n        ticks = new TicksClient();\n        chat = new ChatClient();\n        ChatService service = construct(ChatService.class);\n        String message = service.say(\"hello\");\n        Assert.equals(true, message.startsWith(\"HTTP_MISSING\"), message);\n    }\n}\n",
        ),
    ];

    fn write(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }
}

#[test]
fn java_test_units_build_stream_clients_and_their_socket_has_no_page() {
    if std::process::Command::new("javac")
        .arg("-version")
        .output()
        .is_err()
    {
        assert!(std::env::var_os("CI").is_none(), "CI must provide javac");
        eprintln!("skipping: javac is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    JavaStreamUnit::write(root.path());
    let report = super::run(root.path(), Path::new("client/tests/ChatServiceTest.java"))
        .expect("native Java test unit");
    assert_eq!(report.cases.len(), 1, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
