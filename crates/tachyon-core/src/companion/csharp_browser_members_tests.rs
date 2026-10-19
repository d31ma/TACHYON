#![allow(clippy::expect_used)]

use super::{CompanionLanguage, owner_table};

struct TaskMembersFixture;

impl TaskMembersFixture {
    const SOURCE: &'static str = "public class NotesService {\n public int Count = 0;\n public async Task<BehaviorJSON> Find(BehaviorJSON key) { return key; }\n public Task Forget(BehaviorJSON key) { return Task.CompletedTask; }\n public int Read() { return Count; }\n private Task Private() { return Task.CompletedTask; }\n}\n";
    const PATH: &'static str = "client/services/NotesService.cs";

    fn table(browser: bool) -> String {
        owner_table(
            CompanionLanguage::CSharp.for_web(browser),
            Self::SOURCE,
            Self::PATH,
            &[],
            "",
            false,
        )
        .expect("validated Task service table")
    }
}

#[test]
fn csharp_browser_tasks_yield_once_and_keep_synchronous_members() {
    let browser = TaskMembersFixture::table(true);
    assert!(browser.contains(
        "TacMember.TaskMethod(async arguments => { return Tac.PlainJson(await Instance.Find("
    ));
    assert!(browser.contains("TacMember.TaskMethod(async arguments => { await Instance.Forget("));
    assert!(browser.contains("return null; }"));
    assert!(!browser.contains("GetAwaiter().GetResult()"));
    assert!(!browser.contains("Instance.Private("));
    let native = TaskMembersFixture::table(false);
    for line in native
        .lines()
        .filter(|line| line.contains("[\"count\"]") || line.contains("[\"read\"]"))
    {
        assert!(browser.lines().any(|other| line == other));
    }
}

#[test]
fn csharp_native_task_tables_use_the_same_task_members_as_the_browser() {
    let native = TaskMembersFixture::table(false);
    let browser = TaskMembersFixture::table(true);
    assert!(native.contains(
        "TacMember.TaskMethod(async arguments => { return Tac.PlainJson(await Instance.Find("
    ));
    assert!(native.contains("TacMember.TaskMethod(async arguments => { await Instance.Forget("));
    assert!(!native.contains("GetAwaiter().GetResult()"));
    assert_eq!(native, browser);
    assert_eq!(
        CompanionLanguage::Kotlin.for_web(true),
        CompanionLanguage::Kotlin
    );
}

#[test]
fn csharp_browser_task_tables_preserve_explicit_member_exclusions() {
    let table = owner_table(
        CompanionLanguage::CSharp.for_web(true),
        TaskMembersFixture::SOURCE,
        TaskMembersFixture::PATH,
        &[String::from("Find")],
        "",
        false,
    )
    .expect("excluded Task member");
    assert!(!table.contains("Instance.Find("));
    assert!(table.contains("Instance.Forget("));
}

#[test]
fn csharp_browser_task_emission_does_not_expand_unsupported_result_types() {
    let source = "public class NotesService {\n public int Count = 0;\n public Task<int> Unsupported() { return null; }\n}\n";
    let table = owner_table(
        CompanionLanguage::CSharp.for_web(true),
        source,
        TaskMembersFixture::PATH,
        &[],
        "",
        false,
    )
    .expect("existing supported field");
    assert!(table.contains("[\"count\"]"));
    assert!(!table.contains("Instance.Unsupported("));
}
