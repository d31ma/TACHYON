#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use tachyon_core::{
    AppFeature, AppTarget, Choice, ProjectChoices, ProjectDiscovery, RealmChoice, Scaffold,
    TacLanguage, YonLanguage,
};

struct StructureCodes;

impl StructureCodes {
    const CHECKED: [u16; 5] = [1310, 1318, 1327, 1363, 1369];
}

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("parent")).expect("directories");
    fs::write(path, text).expect("source");
}

fn scaffolded(choices: &ProjectChoices) -> (tempfile::TempDir, PathBuf) {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path().join("app");
    Scaffold::create_with(&root, Some("Structure"), choices).expect("scaffold");
    (workspace, root)
}

fn choices(realm: RealmChoice, tac: &[TacLanguage], yon: &[YonLanguage]) -> ProjectChoices {
    ProjectChoices {
        realm,
        tac: tac.to_vec(),
        yon: yon.to_vec(),
        targets: vec![AppTarget::Web],
        features: Vec::new(),
    }
}

fn problems(root: &Path) -> Vec<(u16, String, String)> {
    ProjectDiscovery::discover(root).map_or_else(
        |failure| {
            failure
                .diagnostics()
                .iter()
                .map(|found| {
                    let file = found
                        .spans
                        .first()
                        .map(|span| span.file.clone())
                        .unwrap_or_default();
                    (found.code.number(), file, found.message.clone())
                })
                .collect()
        },
        |_| Vec::new(),
    )
}

fn structure_problems(root: &Path) -> Vec<(u16, String, String)> {
    problems(root)
        .into_iter()
        .filter(|(code, _, _)| StructureCodes::CHECKED.contains(code))
        .collect()
}

fn assert_every_project_passes(projects: Vec<(String, ProjectChoices)>) {
    for (name, picked) in projects {
        let (_workspace, root) = scaffolded(&picked);
        let found = problems(&root);
        assert!(found.is_empty(), "{name}: {found:#?}");
    }
}

#[test]
fn every_init_language_and_feature_passes_the_structure_validation() {
    let mut every = choices(RealmChoice::Both, TacLanguage::ALL, YonLanguage::ALL);
    every.features = AppFeature::ALL.to_vec();
    assert_every_project_passes(vec![("every language and feature".to_owned(), every)]);
}

#[test]
fn each_tac_language_alone_passes_the_structure_validation() {
    let projects = TacLanguage::ALL
        .iter()
        .map(|language| {
            let picked = choices(RealmChoice::Client, &[*language], &[]);
            (format!("tac {}", language.flag_name()), picked)
        })
        .collect();
    assert_every_project_passes(projects);
}

#[test]
fn each_yon_language_alone_passes_the_structure_validation() {
    let projects = YonLanguage::ALL
        .iter()
        .map(|language| {
            let picked = choices(RealmChoice::Server, &[], &[*language]);
            (format!("yon {}", language.flag_name()), picked)
        })
        .collect();
    assert_every_project_passes(projects);
}

#[test]
fn one_run_reports_every_structure_problem_in_nested_role_folders_and_languages() {
    let (_workspace, root) = scaffolded(&ProjectChoices::defaults());
    let invalid = [
        (
            1363,
            "client/services/billing/BillingThing.ts",
            "export default class BillingThing {\n  total: number = 0\n}\n",
        ),
        (
            1363,
            "client/stores/cart/CartThing.js",
            "export default class CartThing {\n  _total = 0\n}\n",
        ),
        (
            1363,
            "client/remotes/api/ApiThing.dart",
            "class ApiThing {\n  final int _total = 0;\n}\n",
        ),
        (
            1363,
            "client/delegates/text/text_thing.rs",
            "pub struct TextThing {\n    total: i32,\n}\n",
        ),
        (
            1363,
            "server/converters/money/MoneyThing.py",
            "class MoneyThing:\n    def _total(self) -> int:\n        return 0\n",
        ),
        (
            1363,
            "server/setups/boot/BootThing.go",
            "type BootThing struct {\n\ttotal int\n}\n",
        ),
        (
            1363,
            "server/entities/user/UserThing.kt",
            "class UserThing {\n    private val total: Int = 0\n}\n",
        ),
        (
            1363,
            "server/controllers/admin/AdminThing.cs",
            "public class AdminThing\n{\n    private int total = 0;\n}\n",
        ),
        (
            1310,
            "server/services/mail/Mail.rb",
            "#[Service]\nclass Mail\n  # @return [Integer]\n  def _total\n    0\n  end\nend\n",
        ),
        (
            1310,
            "server/delegates/format/FormatDelegate.lua",
            "-- @Delegate\nFormatterDelegate = {}\nFormatterDelegate.__index = FormatterDelegate\n\nfunction FormatterDelegate:_total()\n  return 0\nend\n",
        ),
        (
            1310,
            "server/services/pair/PairService.java",
            "@Service\npublic class PairService {\n    private int total = 0;\n}\nclass Extra {\n    private int total = 0;\n}\n",
        ),
        (
            1369,
            "client/services/link/LinkService.js",
            "import { HomeController } from '../../../server/controllers/HomeController.js'\n@Service\nexport default class LinkService {\n  _home = String(HomeController)\n}\n",
        ),
        (
            1327,
            "server/utils/LabelUtils.ts",
            "export default class LabelUtils {\n  static label(home: HomeController): string {\n    return String(home)\n  }\n}\n",
        ),
    ];
    for (_, path, source) in invalid {
        write(&root, path, source);
    }
    let found = structure_problems(&root);
    for (code, path, _) in invalid {
        assert!(
            found
                .iter()
                .any(|(found_code, file, _)| *found_code == code && *file == path),
            "TY{code} for {path} is missing: {found:#?}"
        );
    }
}

#[test]
fn models_utilities_tests_assets_and_folders_without_a_role_need_no_marker() {
    let (_workspace, root) = scaffolded(&ProjectChoices::defaults());
    for (path, source) in [
        (
            "server/models/billing/Invoice.ts",
            "export interface Invoice {\n  total: number\n}\n",
        ),
        (
            "client/models/cart/CartLine.ts",
            "export interface CartLine {\n  quantity: number\n}\n",
        ),
        (
            "client/shared/utils/text/TextUtils.ts",
            "export default class TextUtils {\n  static trim(value: string): string {\n    return value.trim()\n  }\n}\n",
        ),
        (
            "server/utils/TimeUtils.py",
            "class TimeUtils:\n    @staticmethod\n    def seconds(value: int) -> str:\n        return str(value)\n",
        ),
        (
            "server/commands/ReportCommand.js",
            "export default class ReportCommand {\n  run() { return 1 }\n}\n",
        ),
        (
            "client/shared/scripts/ThemeDefault.js",
            "document.documentElement.dataset.theme = 'light'\n",
        ),
        (
            "client/shared/assets/chart/chart.min.js",
            "var chart=function(){return 1};\n",
        ),
    ] {
        write(&root, path, source);
    }
    let found = structure_problems(&root);
    assert!(found.is_empty(), "{found:#?}");
}

fn new_refusal((code, file, message): &(u16, String, String)) -> bool {
    let utility_scope = file.starts_with("client/shared/utils/") && *code != 1310;
    let utility_need = message.contains("is a utility, and it names");
    [*code == 1369, utility_scope, utility_need].contains(&true)
}

#[test]
fn the_website_passes_the_structure_validation() {
    let website = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../website");
    let found = problems(&website);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_new_refusals_find_nothing_in_the_test_fixtures() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let projects = fs::read_dir(fixtures).expect("fixtures");
    for project in projects.map(|entry| entry.expect("fixture").path()) {
        let found: Vec<_> = problems(&project).into_iter().filter(new_refusal).collect();
        assert!(found.is_empty(), "{}: {found:#?}", project.display());
    }
}

#[test]
fn a_tac_app_with_nested_native_rust_services_and_shared_utilities_passes_unchanged() {
    let picked = choices(
        RealmChoice::Client,
        &[TacLanguage::JavaScript, TacLanguage::Rust],
        &[],
    );
    let (_workspace, root) = scaffolded(&picked);
    for (path, source) in [
        (
            "client/services/native/sessions/SessionReaderService.rs",
            "#[Service]\nstruct SessionReaderService {\n    label: String,\n}\n\nimpl SessionReaderService {\n    fn new() -> Self {\n        Self { label: TimeFormatUtils::seconds(3) }\n    }\n\n    fn refresh(&mut self) {\n        self.label = TimeFormatUtils::seconds(4);\n    }\n}\n",
        ),
        (
            "client/shared/utils/time/time_format_utils.rs",
            "pub struct TimeFormatUtils;\n\nimpl TimeFormatUtils {\n    pub fn seconds(count: u64) -> String {\n        format!(\"{count} s\")\n    }\n}\n",
        ),
        (
            "client/services/browser/ObservatoryService.js",
            "@Service\nexport default class ObservatoryService {\n  _title = ChartDataUtils.title('Observatory')\n}\n",
        ),
        (
            "client/shared/utils/ChartDataUtils.js",
            "export default class ChartDataUtils {\n  /** @param {string} name @returns {string} */\n  static title(name) {\n    return name.trim()\n  }\n}\n",
        ),
        (
            "client/models/TaskRecord.ts",
            "export interface TaskRecord {\n  name: string\n  seconds: number\n}\n",
        ),
    ] {
        write(&root, path, source);
    }
    let found = problems(&root);
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn a_go_yon_backend_for_frontend_with_nested_roles_and_native_packages_passes_unchanged() {
    let (_workspace, root) = scaffolded(&choices(RealmChoice::Server, &[], &[YonLanguage::Go]));
    for (path, source) in [
        (
            "server/controllers/GoGreetingController.go",
            "// @RequestRoute(\"/api/go\")\n// @Public\ntype GoGreetingController struct {\n\tsessionService *SessionService\n}\n\n// @GetRoute\nfunc (controller *GoGreetingController) GetGreeting() string {\n\treturn controller.sessionService.Label()\n}\n",
        ),
        (
            "server/services/session/SessionService.go",
            "// @Service\ntype SessionService struct {\n\tproxyBodyDelegate *ProxyBodyDelegate\n}\n\nfunc (service *SessionService) Label() string {\n\treturn service.proxyBodyDelegate.Body(DiffUtils{}.Changed(1, 2))\n}\n",
        ),
        (
            "server/delegates/proxy/ProxyBodyDelegate.go",
            "// @Delegate\ntype ProxyBodyDelegate struct {\n}\n\nfunc (helper *ProxyBodyDelegate) Body(changed bool) string {\n\tif changed {\n\t\treturn \"changed\"\n\t}\n\treturn \"same\"\n}\n",
        ),
        (
            "server/utils/diff/DiffUtils.go",
            "type DiffUtils struct{}\n\nfunc (DiffUtils) Changed(before int, after int) bool {\n\treturn before != after\n}\n",
        ),
        (
            "server/packages/go.mod",
            "module example.com/heimdall/packages\n\ngo 1.27\n",
        ),
        (
            "server/packages/helpers/git/repository.go",
            "package git\n\ntype Repository struct{ Path string }\n\nfunc (repository *Repository) Head() string {\n\treturn repository.Path\n}\n",
        ),
        (
            "server/packages/services/diff/report_builder.go",
            "package diff\n\ntype ReportBuilder struct{}\n\ntype Report struct{ Lines int }\n\nfunc (ReportBuilder) Build() Report {\n\treturn Report{}\n}\n",
        ),
        (
            "server/models/proxy/ProxySettings.go",
            "type ProxySettings struct {\n\tHost string\n\tPort int\n}\n",
        ),
    ] {
        write(&root, path, source);
    }
    let found = problems(&root);
    assert!(found.is_empty(), "{found:#?}");
}
