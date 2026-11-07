#![allow(clippy::expect_used)]

use std::path::Path;

struct HelperCallerFiles;

impl HelperCallerFiles {
    const HELPER: &'static str = "using System;\n[Delegate]\nclass StepDelegate {\n private readonly string label;\n public StepDelegate(string label) { this.label = label; }\n public string Read() { return label; }\n}\n";

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
            "client/delegates/deep/StepDelegate.cs",
            Self::HELPER,
        );
        let roles = crate::behavior::ServiceRoles::read(entry, service).expect("Service roles");
        super::assemble(root.path(), entry, &roles.source, "tac")
    }
}

#[test]
fn csharp_helper_caller_failure_retains_nested_authored_marker_location() {
    let source = "using System;\n[Service]\nclass TallyService {\n private readonly StepDelegate helper;\n private readonly string sample = \"new StepDelegate()\";\n // new StepDelegate(\"comment\");\n}\n";
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
        "client/delegates/deep/StepDelegate.cs"
    );
    assert_eq!(diagnostic.spans[0].start, 14);
    assert_eq!(diagnostic.spans[0].end, 15);
}

#[test]
fn csharp_helper_a_service_does_not_name_stays_out_of_its_build() {
    let source = "using System;\n[Service]\nclass TallyService {\n private readonly string sample = \"new StepDelegate()\";\n // new StepDelegate(\"comment\");\n public string Read() { return sample; }\n}\n";
    let source = HelperCallerFiles::assemble(source)
        .expect("a Delegate another service needs does not stop this one")
        .unwrap_or_default();
    assert!(!source.contains("class StepDelegate"), "{source}");
}

#[test]
fn csharp_helper_real_service_caller_is_checked_across_captured_files() {
    let source = "using System;\n[Service]\nclass TallyService {\n private readonly StepDelegate helper = new StepDelegate(\"owned\");\n public string Read() { return helper.Read(); }\n}\n";
    let source = HelperCallerFiles::assemble(source)
        .expect("real external caller")
        .expect("captured Delegate");
    assert!(source.contains("new StepDelegate(\"owned\")"));
    assert!(source.contains("class StepDelegate"));
}

#[test]
fn csharp_and_fsharp_needs_fields_own_the_helpers_they_name() {
    for (extension, helper, owner, service, written) in [
        (
            "cs",
            "[Delegate]\npublic sealed class RateDelegate {\n public int Rate() { return 2; }\n}\n",
            "[Delegate]\npublic sealed class TaxDelegate {\n [Needs]\n private readonly RateDelegate _rates;\n public int Add(int value) { return value + _rates.Rate(); }\n}\n",
            "[Service]\npublic sealed class TallyService {\n [Needs]\n private readonly TaxDelegate _tax;\n public int Read() { return _tax.Add(40); }\n}\n",
            [
                "private readonly RateDelegate _rates = new RateDelegate();",
                "private readonly TaxDelegate _tax = new TaxDelegate();",
            ],
        ),
        (
            "fs",
            "[<Delegate>]\ntype RateDelegate() =\n    member this.Rate() : int = 2\n",
            "[<Delegate>]\ntype TaxDelegate(rates: RateDelegate) =\n    member this.Add(value: int) : int = value + rates.Rate()\n",
            "[<Service>]\ntype TallyService() =\n    [<Needs>]\n    member val Tax : TaxDelegate = Unchecked.defaultof<TaxDelegate> with get, set\n    member this.Read() : int = this.Tax.Add(40)\n",
            [
                "let rates = new RateDelegate()",
                "member val private Tax : TaxDelegate = new TaxDelegate() with get, set",
            ],
        ),
    ] {
        let root = tempfile::tempdir().expect("captured project");
        let entry = format!("client/services/TallyService.{extension}");
        HelperCallerFiles::write(root.path(), &entry, service);
        HelperCallerFiles::write(
            root.path(),
            &format!("client/delegates/RateDelegate.{extension}"),
            helper,
        );
        HelperCallerFiles::write(
            root.path(),
            &format!("client/delegates/tax/TaxDelegate.{extension}"),
            owner,
        );
        let entry = Path::new(&entry);
        let roles = crate::behavior::ServiceRoles::read(entry, service).expect("Service roles");
        let authored = [roles.source.as_str(), service][usize::from(extension == "fs")];
        let source = super::assemble(root.path(), entry, authored, "tac")
            .unwrap_or_else(|failure| panic!("{extension}: {failure}"))
            .expect("captured Delegates");
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
            "client/delegates/TaxDelegate.cs",
            "[Delegate]\n[Lifetime({\"value\":\"prototype\"})]\npublic sealed class TaxDelegate {}\n",
            "[Lifetime",
        ),
        (
            "client/delegates/TaxDelegate.cs",
            "[Delegate]\npublic sealed class TaxDelegate {\n [OnStart]\n public void Open() {}\n}\n",
            "[OnStart]",
        ),
        (
            "client/delegates/TaxDelegate.fs",
            "[<Delegate>]\n[<NeedsAll>]\ntype TaxDelegate() =\n    member this.Read() : int = 1\n",
            "NeedsAll",
        ),
        (
            "client/delegates/TaxDelegate.fs",
            "[<Delegate>]\ntype TaxDelegate() =\n    [<OnStop>]\n    member this.Close() : unit = ()\n",
            "[<OnStop>]",
        ),
        (
            "client/delegates/TaxDelegate.fs",
            "[<Delegate>]\ntype TaxDelegate() =\n    [<Needs>]\n    member val Prices : PriceStore = Unchecked.defaultof<PriceStore> with get, set\n",
            "[<Needs>]",
        ),
    ] {
        let failure = crate::behavior::HelperRole::refuse_in_tac(Path::new(path), source)
            .expect_err("a Delegate has no lifetime, hooks or non-Delegate Needs");
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1308, "{path}: {failure}");
        let start = source.find(marker).expect("marker");
        assert!(
            usize::try_from(diagnostic.spans[0].start).expect("offset") <= start,
            "{path}: {failure}"
        );
    }
}
