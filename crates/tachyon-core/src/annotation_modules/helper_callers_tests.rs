#![allow(clippy::expect_used)]

use std::path::Path;

struct HelperCallerFiles;

impl HelperCallerFiles {
    const HELPER: &'static str = "using System;\n[Helper]\nclass StepHelper {\n private readonly string label;\n public StepHelper(string label) { this.label = label; }\n public string Read() { return label; }\n}\n";

    fn write(root: &Path, name: &str, source: &str) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        std::fs::write(path, source).expect("fixture source");
    }

    fn assemble(service: &str) -> Result<Option<String>, crate::Failure> {
        let root = tempfile::tempdir().expect("captured project");
        let entry = Path::new("client/services/TallyService.cs");
        Self::write(root.path(), entry.to_str().expect("entry"), service);
        Self::write(
            root.path(),
            "client/helpers/deep/StepHelper.cs",
            Self::HELPER,
        );
        let roles = crate::behavior::ServiceRoles::read(entry, service).expect("Service roles");
        super::assemble(root.path(), entry, &roles.source, "tac")
    }
}

#[test]
fn csharp_helper_caller_failure_retains_nested_authored_marker_location() {
    let source = "using System;\n[Service]\nclass TallyService {\n private readonly string sample = \"new StepHelper()\";\n // new StepHelper(\"comment\");\n}\n";
    let failure =
        HelperCallerFiles::assemble(source).expect_err("strings/comments cannot provide ownership");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 1308);
    assert!(
        diagnostic
            .message
            .contains("explicit caller-owned construction")
    );
    assert_eq!(
        diagnostic.spans[0].file,
        "client/helpers/deep/StepHelper.cs"
    );
    assert_eq!(diagnostic.spans[0].start, 14);
    assert_eq!(diagnostic.spans[0].end, 15);
}

#[test]
fn csharp_helper_real_service_caller_is_checked_across_captured_files() {
    let source = "using System;\n[Service]\nclass TallyService {\n private readonly StepHelper helper = new StepHelper(\"owned\");\n public string Read() { return helper.Read(); }\n}\n";
    let source = HelperCallerFiles::assemble(source)
        .expect("real external caller")
        .expect("captured Helper");
    assert!(source.contains("new StepHelper(\"owned\")"));
    assert!(source.contains("class StepHelper"));
}

#[test]
fn csharp_and_fsharp_needs_fields_own_the_helpers_they_name() {
    for (extension, helper, owner, service, written) in [
        (
            "cs",
            "[Helper]\npublic sealed class RateHelper {\n public int Rate() { return 2; }\n}\n",
            "[Helper]\npublic sealed class TaxHelper {\n [Needs]\n private readonly RateHelper _rates;\n public int Add(int value) { return value + _rates.Rate(); }\n}\n",
            "[Service]\npublic sealed class TallyService {\n [Needs]\n private readonly TaxHelper _tax;\n public int Read() { return _tax.Add(40); }\n}\n",
            [
                "private readonly RateHelper _rates = new RateHelper();",
                "private readonly TaxHelper _tax = new TaxHelper();",
            ],
        ),
        (
            "fs",
            "[<Helper>]\ntype RateHelper() =\n    member this.Rate() : int = 2\n",
            "[<Helper>]\ntype TaxHelper(rates: RateHelper) =\n    member this.Add(value: int) : int = value + rates.Rate()\n",
            "[<Service>]\ntype TallyService() =\n    [<Needs>]\n    member val Tax : TaxHelper = Unchecked.defaultof<TaxHelper> with get, set\n    member this.Read() : int = this.Tax.Add(40)\n",
            [
                "let rates = new RateHelper()",
                "member val private Tax : TaxHelper = new TaxHelper() with get, set",
            ],
        ),
    ] {
        let root = tempfile::tempdir().expect("captured project");
        let entry = format!("client/services/TallyService.{extension}");
        HelperCallerFiles::write(root.path(), &entry, service);
        HelperCallerFiles::write(
            root.path(),
            &format!("client/helpers/RateHelper.{extension}"),
            helper,
        );
        HelperCallerFiles::write(
            root.path(),
            &format!("client/helpers/tax/TaxHelper.{extension}"),
            owner,
        );
        let entry = Path::new(&entry);
        let roles = crate::behavior::ServiceRoles::read(entry, service).expect("Service roles");
        let authored = [roles.source.as_str(), service][usize::from(extension == "fs")];
        let source = super::assemble(root.path(), entry, authored, "tac")
            .unwrap_or_else(|failure| panic!("{extension}: {failure}"))
            .expect("captured Helpers");
        for part in written {
            assert!(source.contains(part), "{extension} {part}:\n{source}");
        }
        assert!(!source.contains("Needs"), "{extension}:\n{source}");
    }
}

#[test]
fn csharp_and_fsharp_helpers_still_refuse_lifetime_and_hooks() {
    for (path, source, marker) in [
        (
            "client/helpers/TaxHelper.cs",
            "[Helper]\n[Lifetime({\"value\":\"prototype\"})]\npublic sealed class TaxHelper {}\n",
            "[Lifetime",
        ),
        (
            "client/helpers/TaxHelper.cs",
            "[Helper]\npublic sealed class TaxHelper {\n [OnStart]\n public void Open() {}\n}\n",
            "[OnStart]",
        ),
        (
            "client/helpers/TaxHelper.fs",
            "[<Helper>]\n[<NeedsAll>]\ntype TaxHelper() =\n    member this.Read() : int = 1\n",
            "NeedsAll",
        ),
        (
            "client/helpers/TaxHelper.fs",
            "[<Helper>]\ntype TaxHelper() =\n    [<OnStop>]\n    member this.Close() : unit = ()\n",
            "[<OnStop>]",
        ),
        (
            "client/helpers/TaxHelper.fs",
            "[<Helper>]\ntype TaxHelper() =\n    [<Needs>]\n    member val Prices : PriceStore = Unchecked.defaultof<PriceStore> with get, set\n",
            "[<Needs>]",
        ),
    ] {
        let failure = crate::behavior::HelperRole::refuse_in_tac(Path::new(path), source)
            .expect_err("a Helper has no lifetime, hooks or non-Helper Needs");
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1308, "{path}: {failure}");
        let start = source.find(marker).expect("marker");
        assert!(
            usize::try_from(diagnostic.spans[0].start).expect("offset") <= start,
            "{path}: {failure}"
        );
    }
}
