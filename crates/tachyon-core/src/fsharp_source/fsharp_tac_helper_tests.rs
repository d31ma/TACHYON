use super::{FsharpTacHelpers, FsharpView};
use std::path::Path;

struct FsharpTacHelperTests;

impl FsharpTacHelperTests {
    const HELPER: &'static str =
        "[<Helper>]\ntype LabelHelper(label: string) =\n    member this.Read() : string = label\n";
    const SETUP: &'static str = "[<Setup>]\ntype LabelSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        let owned: LabelHelper = new LabelHelper(\"kept\")\n        owned.Read() |> ignore\n        BehaviorAdapters()\n";

    fn callers(helper: &str, caller: &str) -> Result<(), crate::Failure> {
        let source = format!("{helper}\n{caller}");
        FsharpTacHelpers::callers(Path::new("client/services/LabelService.fs"), &source)
    }

    fn source(source: &str) -> Result<(), crate::Failure> {
        FsharpTacHelpers::source(Path::new("client/helpers/nested/LabelHelper.fs"), source)
    }
}

#[test]
fn fsharp_tac_helper_primary_constructor_and_zero_arg_adapters_keep_real_ownership() {
    for (helper, caller) in [
        (FsharpTacHelperTests::HELPER, FsharpTacHelperTests::SETUP),
        (
            "[<Helper>]\ntype LabelHelper() =\n    interface BehaviorHTTPAdapter with\n        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =\n            Task.FromResult(request)\n",
            "[<Setup>]\ntype LabelSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        let adapters: BehaviorAdapters = BehaviorAdapters()\n        adapters.http.Add(\"probe\", new LabelHelper())\n        adapters\n",
        ),
    ] {
        FsharpTacHelperTests::source(helper)
            .unwrap_or_else(|error| panic!("raw ownership: {error}"));
        FsharpTacHelperTests::callers(helper, caller)
            .unwrap_or_else(|error| panic!("raw/shadow caller: {error}"));
        let source = format!("{helper}\n{caller}");
        let view = FsharpView::tac(Path::new("client/setups/LabelSetup.fs"), &source);
        let roles = crate::behavior::ServiceRoles::read(view.path(), view.text())
            .unwrap_or_else(|error| panic!("roles: {error}"));
        let lowered = crate::behavior::detached_csharp(view.path(), &roles.source, "tac")
            .unwrap_or_else(|error| panic!("interface/constructor: {error}"));
        assert!(lowered.is_some());
        assert!(roles.source.contains("public class LabelHelper"));
    }
}

#[test]
fn fsharp_tac_helper_orphan_self_comments_strings_and_unrelated_types_do_not_prove_callers() {
    for caller in [
        "",
        "// new LabelHelper(\"kept\")\n",
        "(* new LabelHelper(\"kept\") *)\n",
        "[<Setup>]\ntype LabelSetup() =\n    let text: string = \"new LabelHelper(kept)\"\n",
        "[<Setup>]\ntype LabelSetup() =\n    let other = new LabelHelperSuffix()\n",
    ] {
        let error = FsharpTacHelperTests::callers(FsharpTacHelperTests::HELPER, caller)
            .err()
            .unwrap_or_else(|| panic!("real external caller required"));
        assert!(error.to_string().contains("caller-owned"));
    }
    let recursive = "[<Helper>]\ntype LabelHelper() =\n    member this.Recursive() : LabelHelper = new LabelHelper()\n";
    assert!(FsharpTacHelperTests::callers(recursive, "").is_err());
}

#[test]
fn fsharp_tac_helper_raw_guards_survive_shadow_constructor_and_attribute_normalization() {
    for marker in ["Needs", "NeedsAll", "Lifetime({\"value\":\"prototype\"})"] {
        let source = format!(
            "[<Helper>]\n[<{marker}>]\ntype LabelHelper(label: string) =\n    member this.Read() : string = label\n"
        );
        let error = FsharpTacHelperTests::source(&source)
            .err()
            .unwrap_or_else(|| panic!("managed ownership refused before normalization"));
        assert_eq!(
            error.diagnostics()[0].spans[0].file,
            "client/helpers/nested/LabelHelper.fs"
        );
    }
    for marker in ["Needs", "OnStart", "OnStop", "OnEvent"] {
        let source = format!(
            "[<Helper>]\ntype LabelHelper() =\n    [<{marker}>]\n    member val Dependency: BehaviorContext = Unchecked.defaultof<BehaviorContext> with get, set\n"
        );
        let error = FsharpTacHelperTests::source(&source)
            .err()
            .unwrap_or_else(|| panic!("property and hook refused"));
        assert_eq!(
            usize::try_from(error.diagnostics()[0].spans[0].start)
                .unwrap_or_else(|error| panic!("span: {error}")),
            source.find("    [<").unwrap_or(0) + 4
        );
    }
    assert!(FsharpTacHelperTests::source("[<Helper>]\ntype LabelHelper = class end\n").is_err());
}

#[test]
fn fsharp_tac_helper_managed_property_and_primary_constructor_requests_are_refused() {
    for caller in [
        "[<Service>]\ntype LabelService(helper: LabelHelper) =\n    member this.Read() : string = helper.Read()\n",
        "[<Service>]\ntype LabelService() =\n    [<Needs>]\n    member val Helper: LabelHelper = Unchecked.defaultof<LabelHelper> with get, set\n",
        "[<Service>]\n[<NeedsAll>]\ntype LabelService(helper: LabelHelper) =\n    member this.Read() : string = helper.Read()\n",
    ] {
        let caller = format!("{}\n{caller}", FsharpTacHelperTests::SETUP);
        assert!(FsharpTacHelperTests::callers(FsharpTacHelperTests::HELPER, &caller).is_err());
    }
}

#[test]
fn fsharp_tac_helper_nested_folder_class_and_filename_rules_remain_real() {
    for (path, source) in [
        (
            "client/helpers/nested/LabelHelper.fs",
            "[<Service>]\ntype LabelService() =\n    member this.Read() : int = 1\n",
        ),
        (
            "client/helpers/nested/OtherHelper.fs",
            FsharpTacHelperTests::HELPER,
        ),
    ] {
        assert!(crate::annotation_layout::validate(Path::new(path), source).is_err());
    }
    crate::annotation_layout::validate(
        Path::new("client/helpers/nested/LabelHelper.fs"),
        FsharpTacHelperTests::HELPER,
    )
    .unwrap_or_else(|error| panic!("genuine nested Helper: {error}"));
}
