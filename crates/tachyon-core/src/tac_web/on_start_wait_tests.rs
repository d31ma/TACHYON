struct OnStartWaitSources;

impl OnStartWaitSources {
    const MESSAGE: &'static str = "A browser HTTP request or socket send cannot wait in OnStart, a getter or a setter. Publish an event from OnStart and make the request in an @OnEvent method.";
    const OUTSIDE_TASK_ANSWERS: [(&'static str, &'static str); 6] = [
        ("Go", include_str!("../native/prelude_tasks.go")),
        ("Kotlin", include_str!("../native/prelude_web_tasks.kt")),
        (
            "Java",
            include_str!("../native/java_web_services/prelude/TacHttpTickets.java"),
        ),
        (
            "Swift",
            include_str!("../native/swift_web_services/TacHttpTickets.swift"),
        ),
        ("Ruby", include_str!("../tac_ruby/tasks.rb")),
        ("Lua", include_str!("../tac_lua/tasks.lua")),
    ];
    const KOTLIN_HTTP_READER: &'static str =
        include_str!("../native/kotlin_web_services/kotlin_browser_http.kt");
    const KOTLIN_SOCKET_READER: &'static str = include_str!("../native/prelude_web_tasks.kt");
    const SWIFT_HTTP_READER: &'static str =
        include_str!("../native/swift_web_services/BehaviorBrowserHttp.swift");
    const SWIFT_SOCKET_READER: &'static str =
        include_str!("../native/swift_web_services/TacSocketSend.swift");
}

#[test]
fn every_outside_task_answer_names_the_event_pattern() {
    for (language, source) in OnStartWaitSources::OUTSIDE_TASK_ANSWERS {
        assert!(
            source.contains(OnStartWaitSources::MESSAGE),
            "{language} does not explain why OnStart cannot wait"
        );
    }
}

#[test]
fn the_kotlin_readers_keep_the_message_of_the_answer() {
    assert!(
        OnStartWaitSources::KOTLIN_HTTP_READER.contains("error[\"message\"] as? String ?: FAILED")
    );
    assert!(
        OnStartWaitSources::KOTLIN_SOCKET_READER.contains(
            "failure?.get(\"message\") as? String ?: \"The socket message was not sent.\""
        )
    );
}

#[test]
fn the_swift_readers_keep_the_message_of_the_answer() {
    assert!(
        OnStartWaitSources::SWIFT_HTTP_READER.contains("message: try? error.get(\"message\").text")
    );
    assert!(
        OnStartWaitSources::SWIFT_SOCKET_READER
            .contains("message: (failure[\"message\"] ?? nil) as? String")
    );
}
