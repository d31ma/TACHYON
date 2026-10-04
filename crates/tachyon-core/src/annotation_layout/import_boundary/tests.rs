use super::ImportBoundary;
use std::path::Path;

fn refused(path: &str, source: &str) -> Vec<(String, String)> {
    let failure = ImportBoundary::check(Path::new(path), source).expect_err("crosses the realm");
    failure
        .diagnostics()
        .iter()
        .map(|diagnostic| {
            assert_eq!(diagnostic.code.number(), ImportBoundary::CODE);
            let span = &diagnostic.spans[0];
            let start = usize::try_from(span.start).expect("start");
            let end = usize::try_from(span.end).expect("end");
            (diagnostic.message.clone(), source[start..end].to_owned())
        })
        .collect()
}

#[test]
fn a_client_service_that_imports_a_server_controller_is_refused_at_the_path() {
    let source = "import { HomeController } from '../../server/controllers/HomeController.js'\n@Service\nexport default class ShellService {}\n";
    let found = refused("client/services/ShellService.js", source);
    assert_eq!(found.len(), 1);
    let (message, spanned) = &found[0];
    assert!(
        message.contains(
            "imports 'server/controllers/HomeController.js', a file of the Yon (server) realm"
        ),
        "{message}"
    );
    assert_eq!(spanned, "../../server/controllers/HomeController.js");
}

#[test]
fn a_server_controller_that_imports_client_code_is_refused() {
    let source = "import ShellService from '../../client/services/ShellService.js'\n@RequestRoute(\"/\")\nexport class HomeController {}\n";
    let found = refused("server/controllers/HomeController.js", source);
    assert!(
        found[0].0.contains("a file of the Tac (client) realm"),
        "{}",
        found[0].0
    );
}

#[test]
fn every_import_form_in_every_checked_language_is_read_and_all_are_reported() {
    let script = "export * from '../../../server/models/Item.js'\nconst late = await import('../../../server/utils/TextUtils.js')\nconst old = require(\"../../../server/utils/DateUtils.js\")\nimport near from '../../server/Near.js'\n@Service\nexport default class ShellService {}\n";
    let found = refused("client/services/nested/ShellService.ts", script);
    let spanned: Vec<&str> = found.iter().map(|(_, spanned)| spanned.as_str()).collect();
    assert_eq!(found.len(), 3, "{spanned:?}");
    assert!(
        spanned.contains(&"../../../server/models/Item.js"),
        "{spanned:?}"
    );
    let dart = "import '../../server/controllers/HomeController.dart';\n@Service()\nclass ExtraService {}\n";
    let found = refused("client/services/ExtraService.dart", dart);
    assert_eq!(found[0].1, "../../server/controllers/HomeController.dart");
}

#[test]
fn imports_inside_the_realm_packages_comments_and_other_calls_are_allowed() {
    let source = "import TextUtils from '../shared/utils/TextUtils.js'\nimport { html } from 'lit'\n// import x from '../../server/a.js'\nconst list = Array.from('../../server/b.js')\nconst text = 'from ../../server/c.js'\nimport up from '../../../../outside.js'\n@Service\nexport default class ShellService {}\n";
    ImportBoundary::check(Path::new("client/services/ShellService.js"), source)
        .expect("same realm, package and non-import text");
}

#[test]
fn files_outside_a_realm_and_unchecked_languages_are_skipped() {
    let script = "import x from './server/controllers/HomeController.js'\n";
    ImportBoundary::check(Path::new("middleware.js"), script).expect("root middleware");
    let python = "from ..server import controllers\n";
    ImportBoundary::check(Path::new("client/services/ShellService.py"), python)
        .expect("python is not read");
}
