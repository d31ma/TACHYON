#![allow(clippy::expect_used)]

use super::outgoing_loopback::LoopbackProbe;
use tachyon_core::{DevServer, ServerKind};

struct UnconfinedProbe;

impl UnconfinedProbe {
    const FILE: &str = "ProbeController.js";
    const SOURCE: &str = r"@RequestRoute('/probe')
@Public
export class ProbeController {
  @GetRoute
  get() {
    return 'answered'
  }
}
";
}

#[tokio::test]
async fn a_production_server_refuses_to_start_where_handlers_cannot_be_confined() {
    let refusal = LoopbackProbe::production_refusal(UnconfinedProbe::FILE, UnconfinedProbe::SOURCE)
        .await
        .expect("ty start refuses without an operating-system egress layer");
    assert!(refusal.contains("TY2113"), "{refusal}");
    assert!(
        refusal.contains("cannot limit a process's network access without an administrator"),
        "{refusal}"
    );
    assert!(
        refusal.contains("YON_EGRESS_ENFORCEMENT=proxy"),
        "{refusal}"
    );
}

#[tokio::test]
async fn a_development_server_starts_and_answers_where_handlers_cannot_be_confined() {
    let project = tempfile::tempdir().expect("project");
    let controller = project
        .path()
        .join("server/controllers")
        .join(UnconfinedProbe::FILE);
    std::fs::create_dir_all(controller.parent().expect("parent")).expect("directory");
    std::fs::write(&controller, UnconfinedProbe::SOURCE).expect("controller");
    std::fs::write(
        project.path().join("server/application.json"),
        LoopbackProbe::DATA_SOURCE,
    )
    .expect("application");
    DevServer::bind(
        project.path(),
        &LoopbackProbe::options(ServerKind::Development),
    )
    .await
    .expect("ty serve starts with a handler.egress_unenforced warning");
}
