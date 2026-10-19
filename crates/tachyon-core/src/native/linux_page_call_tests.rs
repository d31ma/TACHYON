#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};

struct LinuxPageCallTests;

impl LinuxPageCallTests {
    const SERVICE: &'static str = r#"
use crate::models::{TransferRequest, TransferTicket};

#[Service]
#[derive(Default)]
struct TransferService {
    content: String,
    offsets: Vec<usize>,
}

impl TransferService {
    pub fn open(&mut self, request: TransferRequest) -> TransferTicket {
        let count = usize::try_from(request.byte_count).unwrap_or(0) / request.label.len().max(1);
        self.content = request.label.repeat(count);
        self.offsets = vec![0];
        while *self.offsets.last().unwrap_or(&0) < self.content.len() {
            let start = *self.offsets.last().unwrap_or(&0);
            let mut end = (start + 24576).min(self.content.len());
            while !self.content.is_char_boundary(end) {
                end -= 1;
            }
            self.offsets.push(end);
        }
        TransferTicket { pages: (self.offsets.len() - 1) as i64, bytes: self.content.len() as i64 }
    }

    pub fn page(&self, index: i64) -> String {
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        match (self.offsets.get(index), self.offsets.get(index.wrapping_add(1))) {
            (Some(start), Some(end)) => self.content[*start..*end].to_owned(),
            _ => String::new(),
        }
    }

    pub fn huge(&self) -> String {
        "x".repeat(70_000)
    }
}
"#;
    const REQUEST: &'static str =
        "pub struct TransferRequest {\n    pub byte_count: i64,\n    pub label: String,\n}\n";
    const TICKET: &'static str =
        "pub struct TransferTicket {\n    pub pages: i64,\n    pub bytes: i64,\n}\n";
    const SCRIPT: &'static str = r#"const owner = '00000000-0000-4000-8000-000000000041';
const post = (payload) => window.webkit.messageHandlers.tachyon.postMessage({capability: 'companion.invoke', payload});
const call = async (op, name, args) => JSON.parse(await post(JSON.stringify({route: '/', service: 'transfer', owner, op, name, args})));
const bytes = (text) => new TextEncoder().encode(text).length;
const init = await call('init', '', []);
if (init.error) return 'init ' + init.error;
const results = [];
for (const label of ['"\\', 'é漢😀', 'plain']) {
  const ticket = (await call('call', 'open', [{byte_count: 300000, label}])).value;
  let text = '';
  let largest = 0;
  for (let index = 0; index < ticket.pages; index += 1) {
    const page = (await call('call', 'page', [index])).value;
    largest = Math.max(largest, bytes(page));
    text += page;
  }
  const expected = label.repeat(Math.floor(300000 / bytes(label)));
  results.push(text === expected && bytes(text) === ticket.bytes && largest <= 24576 ? 'ok' : 'bad:' + ticket.pages + ':' + ticket.bytes + ':' + bytes(text) + ':' + largest);
}
const huge = await call('call', 'huge', []);
results.push(String(huge.error || '').includes('pages of at most 24 KiB') ? 'large-refused' : 'large:' + JSON.stringify(huge).slice(0, 80));
let oversized = 'accepted';
try { await post('x'.repeat(70000)); } catch (error) { oversized = String(error.message || error).includes('exceeds its bounds') ? 'request-refused' : 'other:' + error; }
results.push(oversized);
return results.join(',');"#;
    const EXPECTED: &'static str = "ok,ok,ok,large-refused,request-refused";

    fn write(path: &Path, source: &str) -> PathBuf {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("fixture source");
        path.to_path_buf()
    }

    fn stage(root: &Path) -> PathBuf {
        Self::write(
            &root.join("client/models/TransferRequest.rs"),
            Self::REQUEST,
        );
        Self::write(&root.join("client/models/TransferTicket.rs"), Self::TICKET);
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source: Self::write(
                &root.join("client/services/TransferService.rs"),
                Self::SERVICE,
            ),
            service: String::from("transfer"),
            class: String::from("TransferService"),
            web_assembly: false,
            application_provider: false,
        };
        super::rust::stage(&[input], root, "org.tachyon.page-calls")
            .expect("staged Rust services")
            .expect("native Rust source")
    }

    fn fixture() -> String {
        include_str!("linux_page_call_fixture.c")
            .replace(
                "__PAGE_SCRIPT__",
                &super::host::c_string_escape(Self::SCRIPT),
            )
            .replace("__PAGE_EXPECTED__", Self::EXPECTED)
    }
}

#[test]
fn the_page_call_fixture_stages_a_paged_transfer_service_and_a_real_bridge_script() {
    let root = tempfile::tempdir().expect("fixture");
    let source =
        std::fs::read_to_string(LinuxPageCallTests::stage(root.path())).expect("companion");
    assert!(source.contains("struct TransferService"));
    let fixture = LinuxPageCallTests::fixture();
    assert!(fixture.contains("window.webkit.messageHandlers.tachyon.postMessage"));
    assert!(!fixture.contains("__PAGE_"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files and a display; run explicitly on the Linux lane"]
async fn linux_page_calls_page_large_results_through_the_real_webkit_bridge() {
    let root = tempfile::tempdir().expect("fixture");
    let source = LinuxPageCallTests::stage(root.path());
    let executable = super::linux_host_tests::LinuxHostTests::compile_with(
        root.path(),
        &source,
        super::native_menu_config::NativeMenuConfig::default(),
        &LinuxPageCallTests::fixture(),
    )
    .await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated Linux host page calls");
    assert!(output.contains("linux page calls passed"), "{output}");
}
