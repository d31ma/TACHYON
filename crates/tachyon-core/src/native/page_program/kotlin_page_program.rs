use super::shared_page_program::SharedPageProgram;
use super::{PageProgram, PageServices};
use crate::Failure;
use crate::project::NativeCompanion;

pub(super) struct KotlinPageProgram;

impl KotlinPageProgram {
    pub(super) fn lowered(services: PageServices<'_, '_>) -> Result<PageProgram, Failure> {
        SharedPageProgram::lowered(NativeCompanion::Kotlin, services)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::super::super::registry::{self, NativeCompanionInput};
    use crate::project::NativeCompanion;
    use std::path::Path;

    struct KotlinPageCheck;

    impl KotlinPageCheck {
        const SETUP: &'static str = "@Setup\nclass NoteSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n";
        const CLIENT: &'static str = "@HTTPClient({\"path\":\"/notes\",\"setup\":\"NoteSetup\"})\ninterface NoteClient {\n    @GetRequest(\"/count\")\n    suspend fun count(): Int\n}\n";

        fn service(class: &str) -> String {
            format!(
                "@Service\nclass {class} {{\n    @Needs\n    private val noteClient: NoteClient\n    var count: Int = 0\n\n    suspend fun read() {{\n        count = noteClient.count()\n    }}\n}}\n"
            )
        }

        fn write(root: &Path, relative: &str, text: &str) {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, text).expect("file");
        }

        fn source(classes: &[(&str, &str)]) -> String {
            let root = tempfile::tempdir().expect("project");
            Self::write(root.path(), "client/setups/NoteSetup.kt", Self::SETUP);
            Self::write(root.path(), "client/remotes/NoteClient.kt", Self::CLIENT);
            let mut services = Vec::new();
            for (instance, class) in classes {
                let relative = format!("client/services/{class}.kt");
                Self::write(root.path(), &relative, &Self::service(class));
                services.push((
                    (*instance).to_owned(),
                    (*class).to_owned(),
                    root.path().join(relative),
                ));
            }
            let companions =
                NativeCompanionInput::web_services(root.path(), NativeCompanion::Kotlin, &services);
            registry::source(&companions, NativeCompanion::Kotlin)
                .expect("the page lowers")
                .expect("a Kotlin program")
        }
    }

    #[test]
    fn two_kotlin_services_share_one_runtime_and_one_copy_of_each_role() {
        let source =
            KotlinPageCheck::source(&[("writer", "WriterService"), ("reader", "ReaderService")]);
        for once in ["object Behaviors {", "class NoteClient", "class NoteSetup"] {
            assert_eq!(source.matches(once).count(), 1, "{once}:\n{source}");
        }
        for namespace in ["TachyonService0", "TachyonService1"] {
            assert!(
                source.contains(&format!("private object {namespace} {{")),
                "{namespace}:\n{source}"
            );
        }
        let roles = source.find("class NoteClient").expect("client");
        let writer = source
            .find("private object TachyonService0 {\n")
            .expect("writer namespace");
        assert!(roles < writer, "{source}");
        assert!(!source.contains("tachyon-page-program"), "{source}");
    }

    #[test]
    fn one_kotlin_service_keeps_its_members_in_its_own_namespace() {
        let source = KotlinPageCheck::source(&[("writer", "WriterService")]);
        let runtime = source.find("object Behaviors {").expect("runtime");
        let namespace = source
            .find("private object TachyonService0 {")
            .expect("namespace");
        assert!(runtime < namespace, "{source}");
        assert_eq!(source.matches("object Behaviors {").count(), 1, "{source}");
    }
}
