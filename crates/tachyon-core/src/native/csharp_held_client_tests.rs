#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use super::windows::WindowsHostGenerator;
use crate::project::NativeCompanion;
use std::path::Path;
use std::time::Duration;

struct HeldClientFixture;

impl HeldClientFixture {
    const FEATURES: &'static str = "../../website/content/features";
    const FILES: [(&'static str, &'static str); 6] = [
        ("sockets", "client/services/ChatService.cs"),
        ("sockets", "client/models/ChatMessage.cs"),
        ("sockets", "client/remotes/ChatClient.cs"),
        ("realtime", "client/services/StreamService.cs"),
        ("realtime", "client/models/Tick.cs"),
        ("realtime", "client/remotes/TicksClient.cs"),
    ];
    const DRIVER: &'static str =
        include_str!("../../tests/fixtures/native-csharp-held-clients/HeldClientsDriver.cs");
    const PROJECT: &'static str = "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net9.0</TargetFramework><OutputType>Exe</OutputType><Nullable>disable</Nullable><AllowUnsafeBlocks>true</AllowUnsafeBlocks><InvariantGlobalization>true</InvariantGlobalization><EnableDefaultCompileItems>false</EnableDefaultCompileItems></PropertyGroup><ItemGroup><Compile Include=\"TachyonCompanion.cs\" /><Compile Include=\"HeldClientsDriver.cs\" /></ItemGroup></Project>";

    fn staged(root: &Path) -> std::path::PathBuf {
        for (feature, file) in Self::FILES {
            let from = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(Self::FEATURES)
                .join(feature)
                .join(file);
            let to = root.join(file);
            std::fs::create_dir_all(to.parent().expect("parent")).expect("directory");
            std::fs::copy(from, to).expect("feature file");
        }
        let companions =
            [("chat", "ChatService"), ("stream", "StreamService")].map(|(service, class)| {
                NativeCompanionInput {
                    snapshot_root: root.to_path_buf(),
                    language: NativeCompanion::CSharp,
                    source: root.join(format!("client/services/{class}.cs")),
                    service: service.to_owned(),
                    class: class.to_owned(),
                    web_assembly: false,
                    application_provider: false,
                }
            });
        WindowsHostGenerator::staged_companion(
            &companions,
            root,
            ("dev.tachyon.held", Some("https://api.example.com")),
        )
        .expect("stage")
        .expect("C# project")
    }

    fn available() -> bool {
        std::process::Command::new("dotnet")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

#[test]
fn a_linked_csharp_service_describes_its_clients_for_the_page_to_hold() {
    let root = tempfile::tempdir().expect("project");
    let project = HeldClientFixture::staged(root.path());
    let source = std::fs::read_to_string(project.join("TachyonCompanion.cs")).expect("companion");
    for wanted in [
        "[\"__stream_ChatClient_Receive\"] = TacMember.ClientStream(",
        "[\"__stream_TicksClient_Receive\"] = TacMember.ClientStream(",
        "TacSocketTransport.Send ??= Tac.SendNativeSocket;",
        ".BehaviorBrowserSocket.Host ??= Tac.SendSocket;",
    ] {
        assert!(source.contains(wanted), "missing {wanted}");
    }
}

#[test]
fn a_linked_csharp_service_hears_the_page_and_sends_on_its_socket_through_it() {
    if !HeldClientFixture::available() {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let project = HeldClientFixture::staged(root.path());
    std::fs::write(
        project.join("TachyonCompanion.csproj"),
        HeldClientFixture::PROJECT,
    )
    .expect("project file");
    std::fs::write(
        project.join("HeldClientsDriver.cs"),
        HeldClientFixture::DRIVER,
    )
    .expect("driver");
    let mut command = std::process::Command::new("dotnet");
    command.arg("run").arg("--project").arg(&project);
    let output = crate::external_command::run_sync(&mut command, Duration::from_mins(4), 65_536)
        .expect("supervised .NET held client fixture");
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        printed.contains("PASS linked C# services hand their clients to the page"),
        "{printed}"
    );
}
