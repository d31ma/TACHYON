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

    const OWNER_AFTER: &'static str = r#"
const equal = (expected, actual) => { if (expected !== actual) throw new Error(`expected ${expected}, got ${actual}`) };
const clientRequest = () => { throw new Error('unexpected request') };
const events = new TacApplicationEvents({changed() {}, failed() { throw new Error('unexpected lifecycle failure') }});
class TitleService {
  initialize() { this.documentTitle.set('plain owner startup') }
  write(value) { this.documentTitle.set(value) }
}
const first = await createOwner({default: TitleService}, {});
const firstSignal = Owners.managed.get(first)?.signal;
if (!firstSignal) throw new Error('ordinary class owner has no captured lease');
const firstPort = TacDocumentTitle.owned(firstSignal);
events.attach(first, {post_construct:'initialize',resources:[{field:'documentTitle',name:'documentTitle'}]}, {documentTitle:firstPort});
await events.initialize(first);
equal('plain owner startup', document.title);
const second = await createOwner({default: TitleService}, {});
const secondSignal = Owners.managed.get(second).signal;
if (secondSignal === firstSignal) throw new Error('independent constructions share lease');
const current = TacDocumentTitle.owned(secondSignal);
current.set('second owner');
events.dispose(first);
releaseOwner(first);
equal(true, firstSignal.aborted);
equal(false, secondSignal.aborted);
let refused = false;
try { firstPort.set('retired') } catch { refused = true }
equal(true, refused);
equal('second owner', document.title);
current.set('second survives');
equal('second survives', document.title);
releaseOwner(second);
equal(true, secondSignal.aborted);
let failedSignal;
try {
  await createOwner({__tachyon_create_owner__:(_, signal) => { failedSignal = signal; throw new Error('construction refused') }}, {});
  throw new Error('failed factory accepted');
} catch (error) { equal('construction refused', error.message) }
equal(true, failedSignal.aborted);
const failing = await createOwner({default: class extends TitleService {
  initialize() { super.initialize(); throw new Error('startup refused') }
}}, {});
const failingSignal = Owners.managed.get(failing).signal;
const failedPort = TacDocumentTitle.owned(failingSignal);
events.attach(failing, {post_construct:'initialize',resources:[{field:'documentTitle',name:'documentTitle'}]}, {documentTitle:failedPort});
try { await events.initialize(failing); throw new Error('failed startup accepted') }
catch (error) { equal('startup refused', error.message) }
events.dispose(failing);
releaseOwner(failing);
equal(true, failingSignal.aborted);
refused = false;
try { failedPort.set('failed-owner') } catch { refused = true }
equal(true, refused);
console.log('document-title-plain-owner-lifecycle-qualified');
"#;

    fn owners() -> String {
        let source = include_str!("../template/tac-client.js");
        let program = [
            Self::BEFORE,
            Self::section(source, "class Owners {", "class StoredAccessor {"),
            "const NativeState = {owners: new WeakMap()};\n",
            include_str!("../template/tac-document-title.js"),
            include_str!("../template/application-events.js"),
            Self::section(source, "const createOwner =", "const disposeManagedOwner ="),
            Self::section(source, "const disposeNativeInstance =", "if (nativeHost())"),
            Self::OWNER_AFTER,
        ]
        .join("\n");
        Self::execute(&program)
    }

    fn section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
        let from = source.find(start).expect("owned runtime start");
        let until = source[from..].find(end).expect("owned runtime end");
        &source[from..from + until]
    }

    fn run() -> String {
        let program = format!(
            "{}{}{}",
            Self::BEFORE,
            include_str!("../template/tac-document-title.js"),
            Self::AFTER
        );
        Self::execute(&program)
    }

    fn execute(program: &str) -> String {
        let root = tempfile::tempdir().expect("fixture");
        let script = root.path().join("document-title.mjs");
        std::fs::write(&script, program).expect("program");
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

#[test]
fn document_title_plain_class_owners_retire_real_captured_leases() {
    assert!(
        DocumentTitleFixture::owners().contains("document-title-plain-owner-lifecycle-qualified")
    );
}
