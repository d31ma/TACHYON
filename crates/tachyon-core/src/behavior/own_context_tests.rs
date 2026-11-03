use super::lower;
use std::path::Path;

struct OwnContextCase;

impl OwnContextCase {
    const EXPORT: &'static str = "export const __tachyon_own_context__";

    fn lowered(path: &str, source: &str, environment: &str) -> String {
        lower(Path::new(path), source, environment)
            .expect("valid behavior source")
            .expect("lowered behavior source")
    }

    fn service(open: &str) -> String {
        format!(
            "@Service\n@CacheSettings({{\"name\":\"catalog\",\"ttlMs\":4000,\"maxBytes\":4096,\"maxEntries\":2}})\nexport default class CatalogService {{\n  @Needs\n  lifecycle;\n  behaviors = null;\n\n  @OnStart\n  async open() {{\n    {open}\n  }}\n\n  @Cached\n  async find(sku) {{ return {{ sku }}; }}\n}}\n"
        )
    }
}

#[test]
fn a_tac_service_that_opens_its_own_context_exports_the_wait_hook() {
    let source = OwnContextCase::service(
        "this.behaviors = await Behaviors.create(CatalogSetup, { partition: 'catalog', signal: this.lifecycle });",
    );
    let output = OwnContextCase::lowered("client/services/CatalogService.js", &source, "tac");
    assert!(output.contains(OwnContextCase::EXPORT), "{output}");
    let hook = output.find(OwnContextCase::EXPORT).expect("hook");
    let runtime = output.find("const Behaviors = ").expect("runtime");
    assert!(runtime < hook, "{output}");
}

#[test]
fn only_an_assignment_to_this_behaviors_opens_an_own_context() {
    for open in [
        "if (this.behaviors === null) return;",
        "// this.behaviors = later",
        "this.behaviorsSeen = true;",
    ] {
        let source = OwnContextCase::service(open);
        let output = OwnContextCase::lowered("client/services/CatalogService.js", &source, "tac");
        assert!(!output.contains(OwnContextCase::EXPORT), "{open}\n{output}");
    }
}

#[test]
fn yon_and_helper_classes_do_not_export_the_wait_hook() {
    let source = OwnContextCase::service(
        "this.behaviors = await Behaviors.create(CatalogSetup, { partition: 'catalog', signal: this.lifecycle });",
    );
    let yon = OwnContextCase::lowered("server/services/CatalogService.js", &source, "yon");
    assert!(!yon.contains(OwnContextCase::EXPORT), "{yon}");
    let helper = source.replace("export default class", "class");
    let output = OwnContextCase::lowered("client/services/CatalogService.js", &helper, "tac");
    assert!(!output.contains(OwnContextCase::EXPORT), "{output}");
}
