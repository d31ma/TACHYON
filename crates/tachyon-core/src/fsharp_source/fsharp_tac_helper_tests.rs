use super::{FsharpTacHelpers, FsharpView};
use std::path::Path;

struct FsharpTacHelperTests;

impl FsharpTacHelperTests {
    const HELPER: &'static str =
        "[<Delegate>]\ntype LabelDelegate(label: string) =\n    member this.Read() : string = label\n";
    const SETUP: &'static str = "[<Setup>]\ntype LabelSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        let owned: LabelDelegate = new LabelDelegate(\"kept\")\n        owned.Read() |> ignore\n        BehaviorAdapters()\n";

    fn callers(helper: &str, caller: &str) -> Result<(), crate::Failure> {
        let source = format!("{helper}\n{caller}");
        FsharpTacHelpers::callers(Path::new("client/services/LabelService.fs"), &source)
    }

    fn source(source: &str) -> Result<(), crate::Failure> {
        FsharpTacHelpers::source(Path::new("client/delegates/nested/LabelDelegate.fs"), source)
    }

    fn owned(helper: &str, caller: &str) {
        Self::source(helper).unwrap_or_else(|error| panic!("raw ownership: {error}"));
        Self::callers(helper, caller).unwrap_or_else(|error| panic!("raw/shadow caller: {error}"));
        let source = format!("{helper}\n{caller}");
        let view = FsharpView::tac(Path::new("client/setups/LabelSetup.fs"), &source);
        let roles = crate::behavior::ServiceRoles::read(view.path(), view.text())
            .unwrap_or_else(|error| panic!("roles: {error}"));
        let lowered = crate::behavior::detached_csharp(view.path(), &roles.source, "tac")
            .unwrap_or_else(|error| panic!("interface/constructor: {error}"));
        assert!(lowered.is_some());
        assert!(roles.source.contains("public class LabelDelegate"));
    }

    fn refused_caller(caller: &str) {
        let error = Self::callers(Self::HELPER, caller)
            .err()
            .unwrap_or_else(|| panic!("real external caller required"));
        assert!(error.to_string().contains("caller-owned"));
    }

    fn refused_class_marker(marker: &str) {
        let source = format!(
            "[<Delegate>]\n[<{marker}>]\ntype LabelDelegate(label: string) =\n    member this.Read() : string = label\n"
        );
        let error = Self::source(&source)
            .err()
            .unwrap_or_else(|| panic!("managed ownership refused before normalization"));
        assert_eq!(
            error.diagnostics()[0].spans[0].file,
            "client/delegates/nested/LabelDelegate.fs"
        );
    }

    fn refused_member_marker(marker: &str) {
        let source = format!(
            "[<Delegate>]\ntype LabelDelegate() =\n    [<{marker}>]\n    member val Dependency: BehaviorContext = Unchecked.defaultof<BehaviorContext> with get, set\n"
        );
        let error = Self::source(&source)
            .err()
            .unwrap_or_else(|| panic!("property and hook refused"));
        assert_eq!(
            usize::try_from(error.diagnostics()[0].spans[0].start)
                .unwrap_or_else(|error| panic!("span: {error}")),
            source.find("    [<").unwrap_or(0) + 4
        );
    }

    fn refused_dependency(caller: &str) {
        let caller = format!("{}\n{caller}", Self::SETUP);
        assert!(Self::callers(Self::HELPER, &caller).is_err());
    }
}

#[test]
fn fsharp_tac_helper_ordinary_primary_constructor_keeps_real_ownership() {
    FsharpTacHelperTests::owned(FsharpTacHelperTests::HELPER, FsharpTacHelperTests::SETUP);
}

#[test]
fn fsharp_tac_helper_zero_argument_http_adapter_keeps_real_ownership() {
    FsharpTacHelperTests::owned(
        "[<Delegate>]\ntype LabelDelegate() =\n    interface BehaviorHTTPAdapter with\n        member this.exchange(request: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =\n            Task.FromResult(request)\n",
        "[<Setup>]\ntype LabelSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters =\n        let adapters: BehaviorAdapters = BehaviorAdapters()\n        adapters.http.Add(\"probe\", new LabelDelegate())\n        adapters\n",
    );
}

#[test]
fn fsharp_tac_helper_orphan_does_not_prove_a_caller() {
    FsharpTacHelperTests::refused_caller("");
}

#[test]
fn fsharp_tac_helper_line_comment_does_not_prove_a_caller() {
    FsharpTacHelperTests::refused_caller("// new LabelDelegate(\"kept\")\n");
}

#[test]
fn fsharp_tac_helper_block_comment_does_not_prove_a_caller() {
    FsharpTacHelperTests::refused_caller("(* new LabelDelegate(\"kept\") *)\n");
}

#[test]
fn fsharp_tac_helper_string_does_not_prove_a_caller() {
    FsharpTacHelperTests::refused_caller(
        "[<Setup>]\ntype LabelSetup() =\n    let text: string = \"new LabelDelegate(kept)\"\n",
    );
}

#[test]
fn fsharp_tac_helper_unrelated_type_does_not_prove_a_caller() {
    FsharpTacHelperTests::refused_caller(
        "[<Setup>]\ntype LabelSetup() =\n    let other = new LabelDelegateSuffix()\n",
    );
}

#[test]
fn fsharp_tac_helper_self_construction_does_not_prove_a_caller() {
    let recursive = "[<Delegate>]\ntype LabelDelegate() =\n    member this.Recursive() : LabelDelegate = new LabelDelegate()\n";
    assert!(FsharpTacHelperTests::callers(recursive, "").is_err());
}

#[test]
fn fsharp_tac_helper_needs_class_marker_remains_refused() {
    FsharpTacHelperTests::refused_class_marker("Needs");
}

#[test]
fn fsharp_tac_helper_needs_all_class_marker_remains_refused() {
    FsharpTacHelperTests::refused_class_marker("NeedsAll");
}

#[test]
fn fsharp_tac_helper_lifetime_class_marker_remains_refused() {
    FsharpTacHelperTests::refused_class_marker("Lifetime({\"value\":\"prototype\"})");
}

#[test]
fn fsharp_tac_helper_needs_member_marker_retains_the_authored_span() {
    FsharpTacHelperTests::refused_member_marker("Needs");
}

#[test]
fn fsharp_tac_helper_start_member_marker_retains_the_authored_span() {
    FsharpTacHelperTests::refused_member_marker("OnStart");
}

#[test]
fn fsharp_tac_helper_stop_member_marker_retains_the_authored_span() {
    FsharpTacHelperTests::refused_member_marker("OnStop");
}

#[test]
fn fsharp_tac_helper_event_member_marker_retains_the_authored_span() {
    FsharpTacHelperTests::refused_member_marker("OnEvent");
}

#[test]
fn fsharp_tac_helper_missing_primary_constructor_remains_refused() {
    assert!(FsharpTacHelperTests::source("[<Delegate>]\ntype LabelDelegate = class end\n").is_err());
}

#[test]
fn fsharp_tac_helper_secondary_constructor_remains_refused() {
    assert!(FsharpTacHelperTests::source(
        "[<Delegate>]\ntype LabelDelegate(label: string) =\n    new() = LabelDelegate(\"secondary\")\n"
    ).is_err());
}

#[test]
fn fsharp_tac_helper_managed_primary_constructor_request_remains_refused() {
    FsharpTacHelperTests::refused_dependency(
        "[<Service>]\ntype LabelService(helper: LabelDelegate) =\n    member this.Read() : string = helper.Read()\n",
    );
}

#[test]
fn fsharp_tac_helper_managed_property_request_remains_refused() {
    FsharpTacHelperTests::refused_dependency(
        "[<Service>]\ntype LabelService() =\n    [<Needs>]\n    member val Delegate: LabelDelegate = Unchecked.defaultof<LabelDelegate> with get, set\n",
    );
}

#[test]
fn fsharp_tac_helper_managed_needs_all_request_remains_refused() {
    FsharpTacHelperTests::refused_dependency(
        "[<Service>]\n[<NeedsAll>]\ntype LabelService(helper: LabelDelegate) =\n    member this.Read() : string = helper.Read()\n",
    );
}

#[test]
fn fsharp_tac_helper_service_under_nested_helpers_remains_refused() {
    assert!(
        crate::annotation_layout::validate(
            Path::new("client/delegates/nested/LabelDelegate.fs"),
            "[<Service>]\ntype LabelService() =\n    member this.Read() : int = 1\n",
        )
        .is_err()
    );
}

#[test]
fn fsharp_tac_helper_mismatched_filename_under_nested_helpers_remains_refused() {
    assert!(
        crate::annotation_layout::validate(
            Path::new("client/delegates/nested/OtherDelegate.fs"),
            FsharpTacHelperTests::HELPER,
        )
        .is_err()
    );
}

#[test]
fn fsharp_tac_helper_genuine_nested_helper_preserves_folder_and_filename_rules() {
    crate::annotation_layout::validate(
        Path::new("client/delegates/nested/LabelDelegate.fs"),
        FsharpTacHelperTests::HELPER,
    )
    .unwrap_or_else(|error| panic!("genuine nested Delegate: {error}"));
}

#[test]
fn fsharp_tac_helper_member_marker_span_is_exact_in_a_crlf_file() {
    let source = "[<Delegate>]\r\ntype LabelDelegate() =\r\n    member this.Read() : string = \"read\"\r\n    [<Needs>]\r\n    member val Dependency: BehaviorContext = Unchecked.defaultof<BehaviorContext> with get, set\r\n";
    let error = FsharpTacHelperTests::source(source)
        .err()
        .unwrap_or_else(|| panic!("property refused"));
    assert_eq!(
        usize::try_from(error.diagnostics()[0].spans[0].start)
            .unwrap_or_else(|error| panic!("span: {error}")),
        source.find("    [<").unwrap_or(0) + 4
    );
}
