#![allow(clippy::expect_used)]

use std::process::Command;
use std::time::Duration;

struct DocumentTitleFixture;

impl DocumentTitleFixture {
    const BEFORE: &'static str = "globalThis.document = {title: 'original'};\n";
    const AFTER: &'static str = r"
const equal = (expected, actual) => { if (expected !== actual) throw new Error(`expected ${expected}, got ${actual}`) };
const first = new AbortController();
const retained = TacDocumentTitle.owned(first.signal);
retained.set('First — 🚀');
equal('First — 🚀', document.title);
const second = new AbortController();
const current = TacDocumentTitle.owned(second.signal);
current.set('second');
first.abort();
equal('second', document.title);
const refuses = (work) => {
  const before = document.title;
  let refused = false;
  try { work() } catch { refused = true }
  if (!refused) throw new Error('invalid operation accepted');
  equal(before, document.title);
};
refuses(() => retained.set('stale'));
current.set('é'.repeat(512));
equal('é'.repeat(512), document.title);
refuses(() => current.set('é'.repeat(513)));
refuses(() => current.set('\u0000'));
refuses(() => current.set('\ud800'));
refuses(() => current.set('\udc00'));
refuses(() => current.set(4));
refuses(() => current.set());
refuses(() => current.set('a', 'b'));
current.set('');
equal('', document.title);
refuses(() => TacDocumentTitle.owned(undefined).set('unowned'));
const previousDocument = document;
globalThis.document = {title: 'new-document'};
refuses(() => current.set('old-document'));
equal('', previousDocument.title);
TacDocumentTitle.host('host title');
equal('host title', document.title);
refuses(() => TacDocumentTitle.host('bad', 'extra'));
console.log('document-title-owned-bounds-qualified');
";

    fn run() -> String {
        let root = tempfile::tempdir().expect("fixture");
        let script = root.path().join("document-title.mjs");
        std::fs::write(
            &script,
            format!(
                "{}{}{}",
                Self::BEFORE,
                include_str!("../template/tac-document-title.js"),
                Self::AFTER
            ),
        )
        .expect("program");
        let output = crate::external_command::run_sync(
            Command::new("node").arg(script),
            Duration::from_secs(20),
            65_536,
        )
        .expect("supervised Node");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

#[test]
fn document_title_page_provider_preserves_captured_ownership_and_bounds() {
    assert!(DocumentTitleFixture::run().contains("document-title-owned-bounds-qualified"));
}
