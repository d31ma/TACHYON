#![allow(clippy::expect_used)]

use std::path::Path;
use std::time::Duration;

struct CsharpBrowserHttpFixture;

impl CsharpBrowserHttpFixture {
    const AUTHORS: [&'static str; 3] = [
        include_str!("../../../tests/fixtures/behavior/csharp-browser-http/CacheProbeService.cs"),
        include_str!("../../../tests/fixtures/behavior/csharp-browser-http/BrowserHttpSetup.cs"),
        include_str!("../../../tests/fixtures/behavior/csharp-browser-http/CustomHttpSetup.cs"),
    ];
    const HARNESS: [&'static str; 5] = [
        include_str!("../../../tests/fixtures/behavior/csharp-browser-http/HttpProbeBindings.cs"),
        include_str!("../../../tests/fixtures/behavior/csharp-browser-http/HttpProbeCache.cs"),
        include_str!("../../../tests/fixtures/behavior/csharp-browser-http/HttpProbeTransport.cs"),
        include_str!("../../../tests/fixtures/behavior/csharp-browser-http/HttpProbeOverride.cs"),
        include_str!("../../../tests/fixtures/behavior/csharp-browser-http/HttpProbeDriver.cs"),
    ];
    const PROJECT: &'static str = "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net9.0</TargetFramework><OutputType>Exe</OutputType><Nullable>disable</Nullable></PropertyGroup></Project>";

    fn source() -> String {
        let path = Path::new("client/services/CacheProbeService.cs");
        let authored = Self::AUTHORS.join("\n");
        let roles =
            crate::behavior::ServiceRoles::read(path, &authored).expect("genuine Service marker");
        crate::behavior::lower(path, &roles.source, "tac")
            .expect("real C# behavior lowering")
            .expect("HTTP/cache runtime")
    }

    fn run() -> String {
        let directory = tempfile::tempdir().expect("isolated HTTP SDK fixture");
        let source = format!("{}\n{}", Self::source(), Self::HARNESS.join("\n"));
        std::fs::write(directory.path().join("Program.cs"), source)
            .expect("emitted Context/runtime");
        let project = directory.path().join("probe.csproj");
        std::fs::write(&project, Self::PROJECT).expect("bounded .NET project");
        let mut command = std::process::Command::new("dotnet");
        command.args(["run", "--project"]).arg(project);
        let result =
            crate::external_command::run_sync(&mut command, Duration::from_mins(2), 65_536)
                .expect("supervised .NET Context fixture");
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8_lossy(&result.stdout).into_owned()
    }
}

#[test]
fn csharp_browser_http_keeps_context_injection_and_real_cache_metadata() {
    let source = CsharpBrowserHttpFixture::source();
    assert!(source.contains("private readonly BehaviorContext behaviors;"));
    assert!(source.contains("public CacheProbeService(BehaviorContext behaviors)"));
    assert!(source.contains("\"cacheable\""));
    assert!(source.contains("\"httpExchange\""));
    assert!(source.contains("browserHttp = BehaviorBrowserHttp.Adapter(dispose)"));
}

#[test]
#[ignore = "requires the installed .NET 9 SDK; controlled network boundary, not Wasm/browser evidence"]
fn csharp_browser_http_contexts_preserve_setup_cache_and_captured_disposal() {
    assert!(
        CsharpBrowserHttpFixture::run()
            .contains("PASS C# owned browser HTTP Context/cache prerequisite")
    );
}
