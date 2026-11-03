#![allow(clippy::expect_used)]

use crate::{BuildOptions, Failure, WebCompiler};

pub(crate) struct FilePickerPage;

impl FilePickerPage {
    const PAGE: (&'static str, &'static str) = ("client/pages/tac.html", "<picker-panel />");
    const PANEL: (&'static str, &'static str) = (
        "client/components/picker/panel/tac.html",
        "<p>{picker.title}</p>",
    );

    pub(crate) fn refusal((path, source): (&str, &str)) -> Failure {
        let root = tempfile::tempdir().expect("project");
        for (relative, text) in [Self::PAGE, Self::PANEL, (path, source)] {
            let file = root.path().join(relative);
            std::fs::create_dir_all(file.parent().expect("folder")).expect("folder");
            std::fs::write(file, text).expect("source");
        }
        WebCompiler::build(root.path(), &BuildOptions::default())
            .expect_err("the page never passes the FilePicker property")
    }
}
