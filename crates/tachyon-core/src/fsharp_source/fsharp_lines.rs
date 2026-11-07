use std::path::Path;

pub(crate) struct FsharpLines;

impl FsharpLines {
    const NO_GAP: &'static str = "";
    const LINE_GAP: &'static str = "\n";

    const LOCATIONS: [fn(&Path, &str) -> String; 2] = [Self::unlocated, Self::located];

    pub(crate) fn located_in(extension: &str, path: &Path, source: &str) -> String {
        Self::LOCATIONS[usize::from(extension == super::FsharpSource::EXTENSION)](path, source)
    }

    fn unlocated(_path: &Path, source: &str) -> String {
        source.to_owned()
    }

    pub(crate) fn hoisted_gap(extension: &str) -> &'static str {
        [Self::NO_GAP, Self::LINE_GAP][usize::from(extension == super::FsharpSource::EXTENSION)]
    }

    pub(crate) fn directive(line: usize, file: &str) -> String {
        format!("# {line} \"{file}\"\n")
    }

    pub(crate) fn located(path: &Path, source: &str) -> String {
        let file = path.to_string_lossy().replace('\\', "/");
        format!("{}{source}", Self::directive(1, &file))
    }

    pub(crate) fn is_directive(trimmed: &str) -> bool {
        trimmed.starts_with('#')
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpLines;
    use std::path::Path;

    #[test]
    fn a_located_module_starts_its_own_line_numbers() {
        assert_eq!(
            FsharpLines::located(Path::new("server/services/A.fs"), "type A() = class end\n"),
            "# 1 \"server/services/A.fs\"\ntype A() = class end\n"
        );
        assert!(FsharpLines::is_directive("# 4 \"server/services/A.fs\""));
        assert!(!FsharpLines::is_directive("member this.A() = 1"));
    }

    #[test]
    fn a_line_directive_inside_a_member_keeps_the_class_open() {
        let source = "[<Service>]\ntype A() =\n    member this.One() : int =\n        BehaviorCalls.Record(\"A.One\")\n# 4 \"server/services/A.fs\"\n        1\n\n    member this.Two() : int = 2\n";
        let shadow = crate::fsharp_source::FsharpShadow::of(source);
        assert!(shadow.text().contains("Two("), "{}", shadow.text());
    }

    #[test]
    fn every_composed_module_keeps_its_own_file_and_line_numbers() {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let service = "open System\n\n[<Service>]\ntype CatalogService() =\n    member this.Total() : int = 2\n";
        let unit = "[<Test(typeof<CatalogTestSetup>)>]\ntype CatalogServiceTest() =\n    member this.testTotals() : unit =\n        Assert.equals(2, CatalogService().Total())\n";
        for (relative, source) in [
            ("server/services/CatalogService.fs", service),
            (
                "server/tests/CatalogTestSetup.fs",
                "[<TestSetup>]\ntype CatalogTestSetup() = class end\n",
            ),
            ("server/tests/CatalogServiceTest.fs", unit),
        ] {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("parent")))
                .unwrap_or_else(|error| panic!("{error}"));
            std::fs::write(path, source).unwrap_or_else(|error| panic!("{error}"));
        }
        let entry = Path::new("server/tests/CatalogServiceTest.fs");
        let composed = crate::annotation_modules::assemble(root.path(), entry, unit, "yon")
            .unwrap_or_else(|failure| panic!("{failure:?}"))
            .unwrap_or_else(|| panic!("composed"));
        assert!(composed.starts_with("open System\n"), "{composed}");
        assert!(
            composed.contains("# 1 \"server/services/CatalogService.fs\"\n\n\n[<Service>]\n"),
            "{composed}"
        );
        assert!(
            composed.contains("# 1 \"server/tests/CatalogServiceTest.fs\"\n[<Test("),
            "{composed}"
        );
    }
}
