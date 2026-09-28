use super::MigrationFix;
use super::visibility_off::VisibilityOff;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct PrivateCases;

impl PrivateCases {
    const CHANGED: [(&'static str, &'static str, &'static str); 9] = [
        (
            "client/services/CartService.js",
            "@Service\nexport default class CartService {\n  static LIMIT = 3\n  count = 0\n\n  total() {\n    return this.limited(this.count)\n  }\n\n  limited(value) {\n    return Math.min(value, CartService.LIMIT)\n  }\n}\n",
            "@Service\nexport default class CartService {\n  static _LIMIT = 3\n  count = 0\n\n  total() {\n    return this._limited(this.count)\n  }\n\n  _limited(value) {\n    return Math.min(value, CartService._LIMIT)\n  }\n}\n",
        ),
        (
            "server/services/PriceService.ts",
            "@Service\nexport default class PriceService {\n  shown(): number {\n    return this.round(1)\n  }\n\n  public round(value: number): number {\n    return value\n  }\n\n  static spare(): number {\n    return 2\n  }\n}\n",
            "@Service\nexport default class PriceService {\n  shown(): number {\n    return this.round(1)\n  }\n\n  private round(value: number): number {\n    return value\n  }\n\n  private static spare(): number {\n    return 2\n  }\n}\n",
        ),
        (
            "server/services/StockService.py",
            "@Service\nclass StockService:\n    limit: int = 3\n\n    def shown(self) -> int:\n        return self.capped(self.limit)\n\n    def capped(self, value: int) -> int:\n        return min(value, StockService.limit)\n",
            "@Service\nclass StockService:\n    _limit: int = 3\n\n    def shown(self) -> int:\n        return self._capped(self._limit)\n\n    def _capped(self, value: int) -> int:\n        return min(value, StockService._limit)\n",
        ),
        (
            "server/services/LedgerService.go",
            "// @Service\ntype LedgerService struct{}\n\nfunc (service *LedgerService) Shown() int {\n\treturn service.Spare()\n}\n\nfunc (service *LedgerService) Spare() int {\n\treturn 1\n}\n",
            "// @Service\ntype LedgerService struct{}\n\nfunc (service *LedgerService) Shown() int {\n\treturn service.spare()\n}\n\nfunc (service *LedgerService) spare() int {\n\treturn 1\n}\n",
        ),
        (
            "server/services/NoteService.rs",
            "#[Service]\n#[derive(Default)]\nstruct NoteService {}\n\nimpl NoteService {\n    pub fn shown(&self) -> i32 {\n        self.spare()\n    }\n\n    pub(crate) fn spare(&self) -> i32 {\n        1\n    }\n}\n",
            "#[Service]\n#[derive(Default)]\nstruct NoteService {}\n\nimpl NoteService {\n    pub fn shown(&self) -> i32 {\n        self.spare()\n    }\n\n    fn spare(&self) -> i32 {\n        1\n    }\n}\n",
        ),
        (
            "server/services/BookService.rb",
            "#[Service]\nclass BookService\n  def shown\n    spare\n  end\n\n  def spare\n    1\n  end\nend\n",
            "#[Service]\nclass BookService\n  def shown\n    spare\n  end\n\n  private def spare\n    1\n  end\nend\n",
        ),
        (
            "server/services/TaxService.kt",
            "@Service\nclass TaxService {\n    var rate: Int = 2\n\n    fun shown(): Int {\n        return spare()\n    }\n\n    fun spare(): Int {\n        return rate\n    }\n}\n",
            "@Service\nclass TaxService {\n    private var rate: Int = 2\n\n    fun shown(): Int {\n        return spare()\n    }\n\n    private fun spare(): Int {\n        return rate\n    }\n}\n",
        ),
        (
            "client/services/PageService.dart",
            "@Service\nclass PageService {\n  int size = 2;\n  List<int> log = [];\n\n  int shown() {\n    log = [...log, 1];\n    return spare();\n  }\n\n  int spare() {\n    return this.size;\n  }\n}\n",
            "@Service\nclass PageService {\n  int _size = 2;\n  List<int> _log = [];\n\n  int shown() {\n    _log = [..._log, 1];\n    return _spare();\n  }\n\n  int _spare() {\n    return this._size;\n  }\n}\n",
        ),
        (
            "server/services/FeeService.cs",
            "[Service]\npublic class FeeService\n{\n    public int Shown()\n    {\n        return Spare();\n    }\n\n    public int Spare()\n    {\n        return 1;\n    }\n}\n",
            "[Service]\npublic class FeeService\n{\n    public int Shown()\n    {\n        return Spare();\n    }\n\n    private int Spare()\n    {\n        return 1;\n    }\n}\n",
        ),
    ];
    const UNCHANGED: [(&'static str, &'static str); 4] = [
        (
            "client/pages/tac.html",
            "<p>{cart.total()} {cart.count} {page.shown()}</p>\n",
        ),
        (
            "server/services/UserService.js",
            "@Service\n@NeedsAll\nexport default class UserService {\n  priceService\n  stockService\n  ledgerService\n  noteService\n  bookService\n  taxService\n  feeService\n  meterService\n\n  shown() {\n    return [this.priceService.shown(), this.stockService.shown(), this.ledgerService.shown(), this.noteService.shown(), this.bookService.shown(), this.taxService.shown(), this.feeService.shown(), this.meterService.shown()]\n  }\n}\n",
        ),
        (
            "server/controllers/UserController.js",
            "@RequestRoute(\"/users\")\n@Public\n@NeedsAll\nclass UserController {\n  userService\n\n  @GetRoute\n  list() {\n    return this.userService.shown()\n  }\n}\n",
        ),
        (
            "server/services/MeterService.dart",
            "@Service\nclass MeterService {\n  int shown() {\n    int spare = 2;\n    return spare;\n  }\n\n  int spare() {\n    return 1;\n  }\n}\n",
        ),
    ];

    const SERVER_ON: &'static str = r#"{"visibility":{"check":true}}"#;
    const CLIENT_ON: &'static str = r#"{"tac":{"visibility":{"check":true}}}"#;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        fs::write(path, contents).expect("file");
    }

    fn read(root: &Path, relative: &str) -> String {
        fs::read_to_string(root.join(relative)).expect("file")
    }
}

#[test]
fn unused_public_members_become_private_in_each_language_form() {
    let project = TempDir::new().expect("project");
    let root = project.path();
    for (relative, before, _) in PrivateCases::CHANGED {
        PrivateCases::write(root, relative, before);
    }
    for (relative, source) in PrivateCases::UNCHANGED {
        PrivateCases::write(root, relative, source);
    }
    PrivateCases::write(root, "server/application.json", PrivateCases::SERVER_ON);
    PrivateCases::write(root, "client/manifest.json", PrivateCases::CLIENT_ON);
    MigrationFix::plan(root)
        .expect("plan")
        .write(root)
        .expect("write");
    for (relative, _, after) in PrivateCases::CHANGED {
        assert_eq!(PrivateCases::read(root, relative), after, "{relative}");
    }
    for (relative, source) in PrivateCases::UNCHANGED {
        assert_eq!(PrivateCases::read(root, relative), source, "{relative}");
    }
}

#[test]
fn a_realm_that_opts_out_keeps_its_public_members() {
    let project = TempDir::new().expect("project");
    let root = project.path();
    let (relative, before, _) = PrivateCases::CHANGED[3];
    PrivateCases::write(root, relative, before);
    VisibilityOff::write(root);
    MigrationFix::plan(root)
        .expect("plan")
        .write(root)
        .expect("write");
    assert_eq!(PrivateCases::read(root, relative), before);
}

#[test]
#[ignore = "makes unused public members private in the project folders that PRIVATE_ROOTS lists"]
fn convert_listed_projects() {
    let list = std::env::var("PRIVATE_ROOTS").expect("PRIVATE_ROOTS names a file list");
    let roots = fs::read_to_string(list).expect("root list");
    for root in roots.lines().filter(|line| !line.trim().is_empty()) {
        let root = Path::new(root);
        let members = crate::annotation_layout::DesignCheck::every_unused_member(root);
        let mut paths: Vec<&str> = members.iter().map(|member| member.path.as_str()).collect();
        paths.dedup();
        for relative in paths {
            let path = root.join(relative);
            let source = fs::read_to_string(&path).expect("source");
            let planned = super::private_members::PrivateMembers::plan(relative, &source, &members);
            let mut edits = Vec::new();
            for (member_edits, note) in planned {
                println!("{}: {note}", path.display());
                edits.extend(member_edits);
            }
            if edits.is_empty() || !super::edits::TextEdit::are_separate(&edits) {
                continue;
            }
            let staged = path.with_file_name(format!(
                ".{}.private",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("source")
            ));
            fs::write(&staged, super::edits::TextEdit::apply(&source, &edits)).expect("stage");
            fs::rename(&staged, &path).expect("move into place");
        }
    }
}
