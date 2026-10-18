#![allow(clippy::expect_used)]

use super::JavaWebServices;
use super::java_page_program::JavaPageProgram;
use super::java_service::JavaService;
use super::java_service_draft::JavaServiceDraft;
use crate::behavior::{JavaPageRuntime, JavaRuntimeSections};
use std::path::{Path, PathBuf};

struct PageFixture;

impl PageFixture {
    const SETUP: &'static str = "import java.util.Map;\n\n@Setup\nclass NotesSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration(Map.of(), Map.of(), Map.of(), null);\n    }\n}\n";
    const CLIENT: &'static str = "@HTTPClient({\"path\":\"/notes\",\"setup\":\"NotesSetup\"})\ninterface NotesClient {\n    @GetRequest(\"/count\")\n    Object count();\n}\n";
    const WRITER: &'static str = "@Service\nclass WriterService {\n    @Needs\n    private final NotesClient notesClient;\n    Object seen = null;\n\n    void read() {\n        seen = notesClient.count();\n    }\n}\n";
    const READER: &'static str = "import java.util.List;\n\n@Service\nclass ReaderService {\n    @Needs\n    private final NotesClient notesClient;\n    Object seen = null;\n\n    void read() {\n        seen = notesClient.count();\n    }\n}\n";
    const PAGE: &'static str = "tachyon/TacPage.java";

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path(), "client/setups/NotesSetup.java", Self::SETUP);
        Self::write(root.path(), "client/remotes/NotesClient.java", Self::CLIENT);
        Self::write(
            root.path(),
            "client/services/WriterService.java",
            Self::WRITER,
        );
        Self::write(
            root.path(),
            "client/services/ReaderService.java",
            Self::READER,
        );
        root
    }

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(path, text).expect("source");
    }

    fn services(root: &Path) -> Vec<(String, String, PathBuf)> {
        ["Writer", "Reader"]
            .iter()
            .map(|name| {
                (
                    name.to_lowercase(),
                    format!("{name}Service"),
                    root.join(format!("client/services/{name}Service.java")),
                )
            })
            .collect()
    }

    fn file<'a>(sources: &'a [(PathBuf, String)], wanted: &str) -> &'a str {
        sources
            .iter()
            .find(|(path, _)| path == Path::new(wanted))
            .map(|(_, text)| text.as_str())
            .expect("generated file")
    }

    fn declared(sources: &[(PathBuf, String)], declaration: &str) -> usize {
        sources
            .iter()
            .map(|(_, text)| text.matches(declaration).count())
            .sum()
    }

    fn written(class: &str, source: &str) -> JavaServiceDraft {
        JavaServiceDraft {
            service: Self::service(class, source),
            required: Vec::new(),
            events: None,
            held: Vec::new(),
            behavior_source: source.to_owned(),
        }
    }

    fn service(class: &str, source: &str) -> JavaService {
        JavaService {
            instance: class.to_lowercase(),
            class: class.to_owned(),
            members: Vec::new(),
            required: Vec::new(),
            providers: Vec::new(),
            path: PathBuf::from(format!("client/services/{class}.java")),
            source: source.to_owned(),
        }
    }

    fn runtime() -> JavaPageRuntime {
        JavaPageRuntime::new(
            Path::new("client/services/One.java"),
            JavaRuntimeSections::default(),
        )
    }

    fn shared(written: &[(&str, &str, &str)]) -> Option<Vec<String>> {
        let drafts: Vec<_> = written
            .iter()
            .map(|(class, authored, _)| Self::written(class, authored))
            .collect();
        let compiled = written
            .iter()
            .map(|(class, _, compiled)| Self::service(class, compiled))
            .collect();
        let built = JavaPageProgram::shared(compiled, &drafts, &Self::runtime())?;
        let mut texts: Vec<String> = built
            .services
            .into_iter()
            .map(|service| service.source)
            .collect();
        texts.extend(built.page.map(|(_, text)| text));
        Some(texts)
    }
}

#[test]
fn two_services_on_a_page_get_one_setup_client_and_runtime() {
    let root = PageFixture::project();
    let sources = JavaWebServices::sources(root.path(), &PageFixture::services(root.path()))
        .expect("sources");
    let page = PageFixture::file(&sources, PageFixture::PAGE);
    assert!(page.contains(" class TacPage { "), "{page}");
    for declaration in [
        "class NotesSetup",
        "class NotesClient",
        "final static class Behaviors",
        "static final String MODULE =",
    ] {
        assert_eq!(
            PageFixture::declared(&sources, declaration),
            1,
            "{declaration}"
        );
        assert!(page.contains(declaration), "{declaration} in {page}");
    }
    let writer = PageFixture::file(&sources, "client/services/WriterService.java");
    let reader = PageFixture::file(&sources, "client/services/ReaderService.java");
    assert!(writer.contains("final class TacService0 extends TacPage { "));
    assert!(reader.contains("final class TacService1 extends TacPage { "));
    assert!(writer.contains("static class WriterService"));
    assert!(!writer.contains("class ReaderService"));
    assert!(page.contains("import java.util.Map;"));
    assert!(page.contains("import java.util.List;"));
}

#[test]
fn a_shared_type_written_two_ways_keeps_each_service_separate() {
    let first = "final class One {}\nfinal class Role { int a; }\n";
    let second = "final class Two {}\nfinal class Role { int b; }\n";
    let shared = PageFixture::shared(&[
        ("One", "final class One {}\n", first),
        ("Two", "final class Two {}\n", second),
    ]);
    assert!(shared.is_none());
}

#[test]
fn imports_that_name_two_types_alike_keep_each_service_separate() {
    let first = "import java.util.List;\nfinal class One {}\nfinal class Role {}\n";
    let second = "import java.awt.List;\nfinal class Two {}\nfinal class Role {}\n";
    let shared = PageFixture::shared(&[
        ("One", "final class One {}\n", first),
        ("Two", "final class Two {}\n", second),
    ]);
    assert!(shared.is_none());
}

#[test]
fn equal_types_move_once_and_blank_their_lines_in_each_service() {
    let first = "final class One {}\nfinal class Role {\n    int a;\n}\n";
    let second = "final class Two {}\nfinal class Role {\n    int a;\n}\n";
    let texts = PageFixture::shared(&[
        ("One", "final class One {}\n", first),
        ("Two", "final class Two {}\n", second),
    ])
    .expect("shared");
    assert_eq!(texts.len(), 3);
    assert!(!texts[0].contains("Role"), "{}", texts[0]);
    assert_eq!(texts[0].lines().count(), first.lines().count() + 1);
    assert!(texts[0].contains("final class TacService0 extends TacPage { "));
    assert_eq!(texts[2].matches("final static class Role").count(), 1);
}

#[test]
fn services_without_common_types_stay_plain_holders() {
    let texts = PageFixture::shared(&[
        ("One", "final class One {}\n", "final class One {}\n"),
        ("Two", "final class Two {}\n", "final class Two {}\n"),
    ])
    .expect("plain");
    assert_eq!(texts.len(), 2);
    assert!(texts[0].contains("final class TacService0 { "));
}
