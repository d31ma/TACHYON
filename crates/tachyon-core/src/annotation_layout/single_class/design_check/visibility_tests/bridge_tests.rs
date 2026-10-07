use super::VisibilityProject;

struct BridgeCases;

impl BridgeCases {
    const CSHARP: &'static str = "[Service]\npublic class BridgeService\n{\n    public int Count = 0;\n\n    public int Doubled() => Count * 2;\n\n    public int Spare() => 3;\n}\n";
    const SCRIPT: &'static str = "@Service\nexport default class BridgeService {\n  verify() {\n    this.count = 2;\n    return this.doubled();\n  }\n\n  _inspect(other) {\n    return other.spare();\n  }\n}\n";
    const VIEW: &'static str = "<button on:click=\"bridge.verify()\">Verify</button>\n";
}

#[test]
fn ty1333_script_siblings_keep_only_the_csharp_members_they_reach() {
    for extension in ["js", "ts"] {
        for folder in ["client/services", "client/services/nested"] {
            let native = format!("{folder}/BridgeService.cs");
            let script = format!("{folder}/BridgeService.{extension}");
            let files = [
                (native.as_str(), BridgeCases::CSHARP),
                (script.as_str(), BridgeCases::SCRIPT),
                ("client/components/bridge/tac.html", BridgeCases::VIEW),
            ];
            assert_eq!(
                VisibilityProject::names_in(&files, &native),
                ["Spare"],
                "{extension}: {folder}"
            );
        }
    }
}

#[test]
fn ty1333_sibling_bridge_reach_is_limited_to_client_services() {
    for folder in ["server/services", "client/helpers", "server/helpers"] {
        let name = if folder.ends_with("helpers") {
            "BridgeHelper"
        } else {
            "BridgeService"
        };
        let native = format!("{folder}/{name}.cs");
        let script = format!("{folder}/{name}.js");
        let csharp = BridgeCases::CSHARP.replace("Service", "Helper");
        let browser = BridgeCases::SCRIPT.replace("Service", "Helper");
        let (csharp, browser) = if folder.ends_with("helpers") {
            (csharp.as_str(), browser.as_str())
        } else {
            (BridgeCases::CSHARP, BridgeCases::SCRIPT)
        };
        let files = [(native.as_str(), csharp), (script.as_str(), browser)];
        assert_eq!(
            VisibilityProject::names_in(&files, &native),
            ["Count", "Doubled", "Spare"],
            "{folder}"
        );
    }
}

#[test]
fn ty1333_a_csharp_service_does_not_keep_its_own_helpers_public() {
    let source = "[Service]\npublic class BridgeService\n{\n    public int Shown() => this.Spare();\n\n    public int Spare() => 3;\n}\n";
    let files = [("client/services/BridgeService.cs", source)];
    assert_eq!(VisibilityProject::names(&files), ["Shown", "Spare"]);
}

#[test]
fn ty1333_a_browser_sibling_does_not_keep_browser_helpers_public() {
    let files = [
        ("client/services/BridgeService.js", BridgeCases::SCRIPT),
        ("client/services/BridgeService.ts", BridgeCases::SCRIPT),
    ];
    assert_eq!(VisibilityProject::names(&files), ["verify", "verify"]);
}

#[test]
fn ty1333_tests_and_other_realms_do_not_count_as_bridge_callers() {
    let files = [
        ("client/services/BridgeService.cs", BridgeCases::CSHARP),
        ("server/services/BridgeService.js", BridgeCases::SCRIPT),
        (
            "client/tests/BridgeServiceTest.js",
            "@Test\nclass BridgeServiceTest { check() { return bridge.count + bridge.doubled(); } }",
        ),
    ];
    assert_eq!(
        VisibilityProject::names_in(&files, "client/services/BridgeService.cs"),
        ["Count", "Doubled", "Spare"]
    );
}

#[test]
fn ty1333_a_fsharp_analysis_view_is_not_a_csharp_bridge_service() {
    let source = "[<Service>]\ntype BridgeService() =\n    member this.Doubled() : int = 2\n    member this.Spare() : int = 3\n";
    let files = [
        ("client/services/BridgeService.fs", source),
        ("client/services/BridgeService.js", BridgeCases::SCRIPT),
    ];
    assert_eq!(
        VisibilityProject::names_in(&files, "client/services/BridgeService.cs"),
        ["Doubled", "Spare"]
    );
}
