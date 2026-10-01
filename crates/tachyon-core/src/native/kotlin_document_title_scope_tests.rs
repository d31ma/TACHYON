#![allow(clippy::expect_used)]

use std::process::Command;
use std::time::Duration;

struct DocumentTitleScopeFixture;

impl DocumentTitleScopeFixture {
    const DRIVER: &'static str = r#"
fun main() {
    val writes = mutableListOf<String>()
    val first = TacPageScope { writes.add(it) }
    val retained = first.documentTitle
    retained.set("First — 🚀")
    val second = TacPageScope { writes.add(it) }
    second.documentTitle.set("second")
    first.dispose()
    check(writes.last() == "second")
    fun refused(work: () -> Unit) {
        val before = writes.toList()
        var rejected = false
        try { work() } catch (_: IllegalStateException) { rejected = true } catch (_: IllegalArgumentException) { rejected = true }
        check(rejected && before == writes)
    }
    refused { retained.set("stale") }
    second.documentTitle.set("é".repeat(512))
    refused { second.documentTitle.set("é".repeat(513)) }
    refused { second.documentTitle.set("\u0000") }
    refused { second.documentTitle.set("\uD800") }
    refused { second.documentTitle.set("\uDC00") }
    second.documentTitle.set("")
    second.dispose()
    val reusedIdentity = TacPageScope { writes.add(it) }
    reusedIdentity.documentTitle.set("replacement")
    refused { second.documentTitle.set("retired") }
    check(writes.last() == "replacement")
    println("document-title-scope-qualified")
}
"#;

    fn run() -> String {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("Scope.kt");
        let jar = root.path().join("scope.jar");
        std::fs::write(
            &source,
            format!(
                "{}{}{}{}{}",
                include_str!("web_apis/DocumentTitle.kt"),
                include_str!("web_apis/DocumentTitleBounds.kt"),
                include_str!("web_apis/TacPageScope.kt"),
                include_str!("web_apis/BrowserDocumentTitle.kt"),
                Self::DRIVER
            ),
        )
        .expect("program");
        Self::execute(
            Command::new("kotlinc")
                .arg(source)
                .arg("-include-runtime")
                .arg("-d")
                .arg(&jar),
        );
        Self::execute(Command::new("java").arg("-jar").arg(jar))
    }

    fn execute(command: &mut Command) -> String {
        let output = crate::external_command::run_sync(command, Duration::from_secs(120), 65_536)
            .expect("supervised SDK");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

#[test]
#[ignore = "requires installed Kotlin/JVM; actual captured provider scope"]
fn kotlin_document_title_scope_retires_once_and_checks_unicode_byte_bounds() {
    assert!(DocumentTitleScopeFixture::run().contains("document-title-scope-qualified"));
}
