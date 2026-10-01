use super::SimplerForms;
use super::receiver_forms::ReceiverForms;
use std::path::Path;

fn rewritten(extension: &str, source: &str) -> String {
    let path = format!("client/services/CounterService.{extension}");
    let mut forms = ReceiverForms::find(Path::new(&path), source);
    assert!(forms.iter().all(|form| form.pattern.code() == 1362));
    forms.sort_by_key(|form| std::cmp::Reverse(form.start));
    forms.into_iter().fold(source.to_owned(), |mut text, form| {
        text.replace_range(form.start..form.end, &form.replacement);
        text
    })
}

#[test]
fn implicit_members_suggests_only_language_permitted_receivers() {
    for (extension, source) in [
        (
            "java",
            "class CounterService {\n int count = 0;\n void increment() { this.count++; }\n}\n",
        ),
        (
            "groovy",
            "class CounterService {\n int count = 0\n void increment() { this.count++ }\n}\n",
        ),
        (
            "cs",
            "class CounterService {\n int count = 0;\n void Increment() { this.count++; }\n}\n",
        ),
        (
            "dart",
            "class CounterService {\n int count = 0;\n void increment() { this.count++; }\n}\n",
        ),
        (
            "kt",
            "class CounterService {\n var count = 0\n fun increment() { this.count++ }\n}\n",
        ),
        (
            "swift",
            "class CounterService {\n var count = 0\n func increment() { self.count += 1 }\n}\n",
        ),
    ] {
        let receiver = if extension == "swift" {
            "self."
        } else {
            "this."
        };
        assert_eq!(
            rewritten(extension, source),
            source.replace(receiver, ""),
            "{extension}"
        );
    }
    for extension in ["js", "ts", "py", "rs", "go", "php", "lua", "fs"] {
        let source =
            "class CounterService {\n int count = 0;\n void increment() { this.count++; }\n}\n";
        assert_eq!(rewritten(extension, source), source);
    }
}

#[test]
fn implicit_members_preserves_shadowing_closures_and_initializing_formals() {
    for (extension, source) in [
        (
            "java",
            "class CounterService {\n int count = 0;\n void set(int count) { this.count = count; }\n}\n",
        ),
        (
            "cs",
            "class CounterService {\n int count = 0;\n void Set() { int count = 3; this.count = count; }\n}\n",
        ),
        (
            "kt",
            "class CounterService {\n var count = 0\n fun set(count: Int) { this.count = count }\n}\n",
        ),
        (
            "groovy",
            "class CounterService {\n int count = 0\n void increment() { run { this.count++ } }\n}\n",
        ),
        (
            "swift",
            "class CounterService {\n var count = 0\n func increment() { later { self.count += 1 } }\n}\n",
        ),
        (
            "dart",
            "class CounterService {\n final int count;\n CounterService(this.count);\n}\n",
        ),
        (
            "java",
            "class CounterService {\n int count = 0;\n void increment() { Outer.this.count++; }\n}\n",
        ),
        (
            "java",
            "class CounterService {\n int count = 0;\n void increment() { this /* keep this explanation */ .count++; }\n}\n",
        ),
    ] {
        assert_eq!(rewritten(extension, source), source, "{extension}");
    }
    let source = "class CounterService {\n int count = 0;\n void increment() { String example = \"this.count\"; this.count++; }\n}\n";
    assert!(rewritten("java", source).contains("\"this.count\"; count++"));
}

#[test]
fn implicit_members_is_off_by_default_and_explicitly_enabled_per_realm() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("client/services")).unwrap();
    let relative = "client/services/CounterService.java";
    let source =
        "class CounterService {\n int count = 0;\n void increment() { this.count++; }\n}\n";
    std::fs::write(root.path().join(relative), source).unwrap();
    assert!(!SimplerForms::report(root.path()).contains("TY1362"));
    assert!(SimplerForms::edits(root.path(), relative, source).is_empty());
    std::fs::write(
        root.path().join("client/manifest.json"),
        r#"{"tac":{"implicitMembers":{"suggest":true}}}"#,
    )
    .unwrap();
    assert!(SimplerForms::report(root.path()).contains("TY1362"));
    assert_eq!(SimplerForms::edits(root.path(), relative, source).len(), 1);
    assert!(
        SimplerForms::edits(root.path(), "server/services/CounterService.java", source).is_empty()
    );
}

#[test]
fn implicit_members_keeps_migration_edits_separate_and_bounded() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("client")).unwrap();
    std::fs::write(
        root.path().join("client/manifest.json"),
        r#"{"tac":{"implicitMembers":{"suggest":true}}}"#,
    )
    .unwrap();
    let relative = "client/services/CounterService.java";
    let source = "class CounterService {\n int count = 0;\n void increment() { this.count = this.count + 1; }\n}\n";
    let forms = SimplerForms::edits(root.path(), relative, source);
    for (index, form) in forms.iter().enumerate() {
        assert!(forms[index + 1..].iter().all(|other| !form.overlaps(other)));
    }
    let repeated = format!(
        "class CounterService {{\n int count = 0;\n void increment() {{ {} }}\n}}\n",
        "this.count++;".repeat(257)
    );
    assert!(ReceiverForms::find(Path::new(relative), &repeated).is_empty());
    assert!(ReceiverForms::find(Path::new("CounterService.min.java"), source).is_empty());
}

#[test]
fn implicit_members_preserves_coverage_hooks_in_each_supported_language() {
    for (extension, source) in [
        (
            "java",
            "@Service\nclass CounterService {\n int count = 0;\n void increment() { count++; }\n}\n",
        ),
        (
            "groovy",
            "@Service\nclass CounterService {\n int count = 0\n void increment() { count++ }\n}\n",
        ),
        (
            "cs",
            "[Service]\nclass CounterService {\n public int count = 0;\n public void increment() { count++; }\n}\n",
        ),
        (
            "dart",
            "@Service\nclass CounterService {\n int count = 0;\n void increment() { count++; }\n}\n",
        ),
        (
            "kt",
            "@Service\nclass CounterService {\n var count = 0\n fun increment() { count++ }\n}\n",
        ),
        (
            "swift",
            "@Service\nclass CounterService {\n var count = 0\n func increment() { count += 1 }\n}\n",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let realm = if extension == "swift" {
            "client"
        } else {
            "server"
        };
        let relative = format!("{realm}/services/CounterService.{extension}");
        std::fs::create_dir_all(root.path().join(format!("{realm}/services"))).unwrap();
        std::fs::write(root.path().join(&relative), source).unwrap();
        let hooked =
            crate::method_coverage::CallHooks::insert(Path::new(&relative), source).unwrap();
        assert!(
            hooked.contains("CounterService.increment"),
            "{extension}: {hooked}"
        );
        let mut calls = crate::method_coverage::RecordedCalls::default();
        calls.add(
            realm,
            "CounterServiceTest.testIncrement",
            &[String::from("CounterService.increment")],
        );
        let report = crate::method_coverage::CoverageReport::build(root.path(), calls).unwrap();
        assert_eq!(report.covered(), 1, "{extension}: {report:?}");
        assert!(report.uncovered().is_empty(), "{extension}: {report:?}");
    }
}

#[test]
fn implicit_members_preserves_injection_and_native_view_contracts() {
    use crate::companion::{CompanionLanguage, owner_table};
    use crate::template::required_fields::RequiredFields;
    let injectable = std::collections::BTreeSet::from([String::from("LedgerStore")]);
    for (extension, source, language) in [
        (
            "java",
            "@NeedsAll\n@Service\nclass CounterService {\n LedgerStore ledgerStore;\n int count = 0;\n void increment() { count += ledgerStore.amount; }\n}\n",
            None,
        ),
        (
            "cs",
            "[NeedsAll]\n[Service]\nclass CounterService {\n [Needs]\n private LedgerStore _ledgerStore;\n public int count = 0;\n public void increment() { count += _ledgerStore.amount; }\n}\n",
            Some(CompanionLanguage::CSharp),
        ),
        (
            "dart",
            "@NeedsAll\n@Service\nclass CounterService {\n final LedgerStore ledgerStore;\n int count = 0;\n void increment() { count += ledgerStore.amount; }\n}\n",
            None,
        ),
        (
            "kt",
            "@NeedsAll\n@Service\nclass CounterService {\n lateinit var ledgerStore: LedgerStore\n var count: Int = 0\n fun increment() { count += ledgerStore.amount }\n}\n",
            Some(CompanionLanguage::Kotlin),
        ),
        (
            "swift",
            "@NeedsAll\n@Service\nclass CounterService {\n var ledgerStore: LedgerStore\n var count: Int = 0\n func increment() { count += ledgerStore.amount }\n}\n",
            Some(CompanionLanguage::Swift),
        ),
    ] {
        let relative = format!("client/services/CounterService.{extension}");
        let receiver = if extension == "swift" { "self" } else { "this" };
        let explicit = source.replace("count +=", &format!("{receiver}.count +="));
        let injected = RequiredFields::injected_fields(
            Path::new(&relative),
            source,
            "CounterService",
            &injectable,
        );
        assert_eq!(injected.len(), 1, "{extension}: {injected:?}");
        assert_eq!(
            injected,
            RequiredFields::injected_fields(
                Path::new(&relative),
                &explicit,
                "CounterService",
                &injectable
            )
        );
        if let Some(language) = language {
            let bare_table = owner_table(language, source, &relative, &injected, "", true).unwrap();
            let explicit_table =
                owner_table(language, &explicit, &relative, &injected, "", true).unwrap();
            assert_eq!(bare_table, explicit_table, "{extension}");
            assert!(bare_table.contains("count") && bare_table.contains("increment"));
        }
        let lowered = crate::behavior::lower(Path::new(&relative), source, "tac")
            .unwrap()
            .unwrap();
        assert!(lowered.contains("count +="), "{extension}: {lowered}");
    }
    let relative = Path::new("server/services/CounterService.groovy");
    let source = "@NeedsAll\n@Service\nclass CounterService {\n LedgerStore ledgerStore\n int count = 0\n void increment() { count += ledgerStore.amount }\n}\n";
    let lowered = crate::behavior::lower(relative, source, "yon")
        .unwrap()
        .unwrap();
    assert!(lowered.contains("count += ledgerStore.amount"), "{lowered}");
    assert!(
        lowered.contains("this.ledgerStore = ledgerStore"),
        "{lowered}"
    );
}
