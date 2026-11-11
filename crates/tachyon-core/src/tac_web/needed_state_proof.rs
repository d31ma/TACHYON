#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use serde_json::json;
use std::path::Path;

pub(crate) struct NeededStateProof {
    pub(crate) language: WebLanguage,
    pub(crate) files: &'static [(&'static str, &'static str)],
    pub(crate) prefix: &'static str,
    pub(crate) setup: fn(&Path),
}

impl NeededStateProof {
    pub(crate) const FETCH: &'static str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n";

    pub(crate) const IMPORT: &'static str = "import { readFile } from 'node:fs/promises'\n";

    const DRIVER: &'static str = "globalThis.location = { pathname: '/' }\n\
         class MemoryStorage {\n\
         \x20 constructor() { this.items = new Map() }\n\
         \x20 getItem(key) { return this.items.has(key) ? this.items.get(key) : null }\n\
         \x20 setItem(key, value) { this.items.set(key, String(value)) }\n\
         \x20 removeItem(key) { this.items.delete(key) }\n\
         }\n\
         const client = await readFile('./tac-client.js', 'utf8')\n\
         const host = client.slice(client.indexOf('const webStorage = '), client.indexOf('const persistScope = '))\n\
         const saved = JSON.parse(await readFile('./saved.json', 'utf8'))\n\
         const { default: EditorService } = await import('./EditorService.js')\n\
         const local = new MemoryStorage()\n\
         const tab = new MemoryStorage()\n\
         local.setItem('tac:service:notes:/:visits', '5')\n\
         tab.setItem('tac:service:notes:/:draft', '\"kept draft\"')\n\
         const settle = () => new Promise((resolve) => setTimeout(resolve, 20))\n\
         const start = async () => {\n\
         \x20 const { NeededServices } = new Function('localStorage', 'sessionStorage', `${host}\\nreturn { NeededServices }`)(local, tab)\n\
         \x20 const editor = new EditorService({})\n\
         \x20 await NeededServices.bind(editor, editor.__tachyonNeeds, saved, NeededServices.holder(editor))\n\
         \x20 return { editor, close: () => { editor.__tachyonDispose(); NeededServices.release(editor) } }\n\
         }\n\
         const first = await start()\n\
         const needs = [...first.editor.__tachyonNeeds]\n\
         const restored = { draft: await first.editor.draft(), visits: await first.editor.visits() }\n\
         await first.editor.write('chapter one')\n\
         await settle()\n\
         const written = { local: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }\n\
         first.close()\n\
         await settle()\n\
         const second = await start()\n\
         const reloaded = { draft: await second.editor.draft(), visits: await second.editor.visits() }\n\
         second.close()\n\
         console.log(JSON.stringify({ needs, restored, written, reloaded }))\n\
         process.exit(0)\n";

    pub(crate) async fn run(&self) -> serde_json::Value {
        let root = tempfile::tempdir().expect("project");
        (self.setup)(root.path());
        self.publish(root.path()).await;
        Self::write_saved(root.path());
        Self::drive(root.path(), &format!("{}{}", self.prefix, Self::DRIVER))
    }

    pub(crate) fn assert_saved(report: &serde_json::Value) {
        assert_eq!(report["needs"], json!(["notes"]), "{report}");
        assert_eq!(
            report["restored"],
            json!({"draft": "kept draft", "visits": 5}),
            "{report}"
        );
        assert_eq!(
            report["written"],
            json!({
                "local": {"tac:service:notes:/:visits": "6"},
                "session": {"tac:service:notes:/:draft": "\"chapter one\""},
            }),
            "{report}"
        );
        assert_eq!(
            report["reloaded"],
            json!({"draft": "chapter one", "visits": 6}),
            "{report}"
        );
    }

    async fn publish(&self, root: &Path) {
        let services = self.services(root);
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        Self::write_all(root, built.files);
        Self::write(
            root,
            "tac-client.js",
            include_str!("../template/tac-client.js").as_bytes(),
        );
    }

    fn services(&self, root: &Path) -> Vec<(String, String, WebLanguage, std::path::PathBuf)> {
        let mut services = Vec::new();
        for (relative, source) in self.files {
            let path = Self::write(root, relative, source.as_bytes());
            services.push((
                Self::instance(relative),
                Self::class(relative),
                self.language,
                path,
            ));
        }
        services
    }

    fn write_all(root: &Path, files: Vec<(String, Vec<u8>)>) {
        for (name, bytes) in files {
            Self::write(root, &name, &bytes);
        }
    }

    fn class(relative: &str) -> String {
        let name = Path::new(relative).file_stem().expect("file name");
        name.to_string_lossy().into_owned()
    }

    fn instance(relative: &str) -> String {
        crate::template::instance_name(&Self::class(relative))
    }

    fn write_saved(root: &Path) {
        let registry = crate::template::ServiceRegistry::discover(root)
            .unwrap_or_else(|failure| panic!("{failure}"));
        let notes = registry.get("notes").expect("notes service").stored();
        assert_eq!(
            json!(notes),
            json!({"draft": "session", "visits": "local"}),
            "the notes service keeps a session draft and a local visit count"
        );
        Self::write(
            root,
            "saved.json",
            json!({ "notes": notes }).to_string().as_bytes(),
        );
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    fn drive(root: &Path, driver: &str) -> serde_json::Value {
        Self::write(root, "drive.mjs", driver.as_bytes());
        let output = NodeDriver::output(NodeDriver::command().arg("drive.mjs").current_dir(root))
            .expect("node");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let last = stdout.lines().last().unwrap_or_default();
        serde_json::from_str(last).unwrap_or_else(|error| panic!("{error}: {stdout}"))
    }
}
