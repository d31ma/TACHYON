use super::WorkerRoot;
use std::path::Path;

#[test]
fn a_worker_without_needs_keeps_its_source() {
    let source = "// @Schedule({\"fixedDelaySeconds\":30})\ntype Handler struct{}\n\nfunc (Handler) POST() {}\n";
    let renamed = WorkerRoot::renamed(Path::new("server/workers/beat.go"), source).expect("worker");
    assert!(renamed.is_none());
}

#[test]
fn a_controller_with_needs_keeps_its_source() {
    let source =
        "@RequestRoute(\"/a\")\nclass AController {\n  @Needs\n  LedgerService ledger;\n}\n";
    let path = Path::new("server/controllers/AController.java");
    assert!(
        WorkerRoot::renamed(path, source)
            .expect("controller")
            .is_none()
    );
}

#[test]
fn a_go_worker_with_a_needs_field_renames_its_handler() {
    let source = "// @Schedule({\"fixedDelaySeconds\":30})\ntype Handler struct {\n\t// @Needs\n\tledger *LedgerService\n}\n\nfunc (worker *Handler) POST() string {\n\treturn \"Handler\" + http.Handler\n}\n";
    let renamed = WorkerRoot::renamed(Path::new("server/workers/beat.go"), source)
        .expect("worker")
        .expect("renamed");
    assert!(renamed.contains("type YonWork struct {"), "{renamed}");
    assert!(
        renamed.contains("func (worker *YonWork) POST()"),
        "{renamed}"
    );
    assert!(renamed.contains("\"Handler\" + http.Handler"), "{renamed}");
    assert_eq!(renamed.len(), source.len());
}

#[test]
fn each_language_spelling_of_needs_marks_an_injecting_worker() {
    let workers = [
        (
            "js",
            "export class Handler {\n  @Needs\n  ledgerService;\n}\n",
        ),
        (
            "rs",
            "struct Handler {\n    #[Needs]\n    ledger: Arc<LedgerService>,\n}\n",
        ),
        (
            "cs",
            "class Handler {\n  [Needs]\n  LedgerService _ledger;\n}\n",
        ),
        (
            "fs",
            "type Handler() =\n    [<Needs>]\n    member val Ledger : LedgerService = Unchecked.defaultof<LedgerService> with get, set\n",
        ),
        (
            "fs",
            "type Handler(ledger: LedgerService) =\n    member this.POST() : int = ledger.Total()\n",
        ),
        (
            "py",
            "class Handler:\n    ledger: Annotated[LedgerService, needs]\n",
        ),
        (
            "lua",
            "Handler = {\n  -- @Needs\n  ---@type LedgerService\n  ledger = nil,\n}\n",
        ),
        (
            "rb",
            "class Handler\n  #[Needs]\n  attr_reader :ledger\nend\n",
        ),
        (
            "kt",
            "class Handler {\n  @yon::NeedsAll\n  val ledger: LedgerService\n}\n",
        ),
    ];
    for (extension, source) in workers {
        let path = format!("server/workers/beat.{extension}");
        assert!(WorkerRoot::injects(Path::new(&path), source), "{extension}");
    }
}

#[test]
fn a_plain_fsharp_worker_does_not_inject() {
    let source = "type Handler() =\n    static member POST() : unit = ()\n";
    assert!(!WorkerRoot::injects(
        Path::new("server/workers/beat.fs"),
        source
    ));
}

#[test]
fn a_worker_that_already_uses_the_generated_name_is_refused() {
    let source = "class Handler {\n  @Needs\n  YonWork ledger;\n}\n";
    let failure = WorkerRoot::renamed(Path::new("server/workers/beat.java"), source)
        .expect_err("refused")
        .to_string();
    assert!(failure.contains("YonWork"), "{failure}");
}
