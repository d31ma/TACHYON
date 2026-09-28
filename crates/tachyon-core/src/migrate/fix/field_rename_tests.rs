use super::MigrationFix;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct RenameCases;

impl RenameCases {
    const RENAMED: [(&'static str, &'static str, &'static str); 7] = [
        (
            "server/services/LedgerService.java",
            "@Service\nclass LedgerService {\n  @Needs\n  LedgerStore LedgerStore;\n\n  int total() {\n    return LedgerStore.subtotal() + this.LedgerStore.count();\n  }\n}\n",
            "@Service\nclass LedgerService {\n  @Needs\n  LedgerStore ledgerStore;\n\n  int total() {\n    return ledgerStore.subtotal() + this.ledgerStore.count();\n  }\n}\n",
        ),
        (
            "server/services/LedgerService.cs",
            "[Service]\nclass LedgerService {\n  [Needs]\n  LedgerStore store;\n\n  public int Total() {\n    return store.Subtotal();\n  }\n}\n",
            "[Service]\nclass LedgerService {\n  [Needs]\n  LedgerStore _store;\n\n  public int Total() {\n    return _store.Subtotal();\n  }\n}\n",
        ),
        (
            "server/services/LedgerService.php",
            "<?php\n#[Service]\nclass LedgerService {\n  #[Needs]\n  private LedgerStore $LedgerStore;\n\n  public function total(): int {\n    return $this->LedgerStore->subtotal();\n  }\n}\n",
            "<?php\n#[Service]\nclass LedgerService {\n  #[Needs]\n  private LedgerStore $ledgerStore;\n\n  public function total(): int {\n    return $this->ledgerStore->subtotal();\n  }\n}\n",
        ),
        (
            "server/services/LedgerService.py",
            "from typing import Annotated\n\n\n@service\nclass LedgerService:\n    LedgerStore: Annotated[LedgerStore, needs]\n\n    def total(self) -> int:\n        return self.LedgerStore.subtotal()\n",
            "from typing import Annotated\n\n\n@service\nclass LedgerService:\n    ledger_store: Annotated[LedgerStore, needs]\n\n    def total(self) -> int:\n        return self.ledger_store.subtotal()\n",
        ),
        (
            "server/services/LedgerService.rb",
            "#[Service]\n#[NeedsAll]\nclass LedgerService\n  attr_reader :ledgerStore\n\n  def total\n    ledgerStore.subtotal + @ledgerStore.count\n  end\nend\n",
            "#[Service]\n#[NeedsAll]\nclass LedgerService\n  attr_reader :ledger_store\n\n  def total\n    ledger_store.subtotal + @ledger_store.count\n  end\nend\n",
        ),
        (
            "server/services/LedgerService.go",
            "package services\n\n// @Service\ntype LedgerService struct {\n\t// @Needs\n\tStore *LedgerStore\n}\n\nfunc (service *LedgerService) Total() int {\n\treturn service.Store.Subtotal()\n}\n",
            "package services\n\n// @Service\ntype LedgerService struct {\n\t// @Needs\n\tstore *LedgerStore\n}\n\nfunc (service *LedgerService) Total() int {\n\treturn service.store.Subtotal()\n}\n",
        ),
        (
            "client/services/CartService.js",
            "@Service\n@NeedsAll\nexport default class CartService {\n  CartStore;\n\n  get count() {\n    return this.CartStore.count();\n  }\n}\n",
            "@Service\n@NeedsAll\nexport default class CartService {\n  cartStore;\n\n  get count() {\n    return this.cartStore.count();\n  }\n}\n",
        ),
    ];
    const UNSURE: (&'static str, &'static str) = (
        "server/services/AuditService.java",
        "@Service\nclass AuditService {\n  @Needs\n  AuditStore AuditStore;\n\n  void record() {\n    send(AuditStore);\n  }\n}\n",
    );
    const VIEW: (&'static str, &'static str, &'static str) = (
        "client/pages/tac.html",
        "<main><p>{cart.CartStore.size}</p><p>{cart.count}</p></main>\n",
        "<main><p>{cart.cartStore.size}</p><p>{cart.count}</p></main>\n",
    );

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
fn injected_fields_get_the_language_case_with_their_uses_and_view_bindings() {
    let project = TempDir::new().expect("project");
    let root = project.path();
    for (relative, before, _) in RenameCases::RENAMED {
        RenameCases::write(root, relative, before);
    }
    let (view, view_before, view_after) = RenameCases::VIEW;
    RenameCases::write(root, view, view_before);
    let (unsure, unsure_source) = RenameCases::UNSURE;
    RenameCases::write(root, unsure, unsure_source);
    MigrationFix::plan(root)
        .expect("plan")
        .write(root)
        .expect("write");
    for (relative, _, after) in RenameCases::RENAMED {
        assert_eq!(RenameCases::read(root, relative), after, "{relative}");
    }
    assert_eq!(RenameCases::read(root, view), view_after);
    assert_eq!(RenameCases::read(root, unsure), unsure_source);
}
