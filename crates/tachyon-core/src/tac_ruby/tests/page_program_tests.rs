use super::{Page, RubyWebServices, write};
use serde_json::json;
use std::path::{Path, PathBuf};

struct PageProgramPage;

impl PageProgramPage {
    const FILES: [(&'static str, &'static str); 3] = [
        (
            "client/models/ProgramState.rb",
            "# @!attribute [r] identity\n#   @return [String]\n# @!attribute [r] count\n#   @return [Integer]\n# @!attribute [r] configured\n#   @return [Integer]\nProgramState = Data.define(:identity, :count, :configured)\n",
        ),
        (
            "client/remotes/PageProgramClient.rb",
            "#[HTTPClient({\"adapter\":\"pageprogram\",\"path\":\"/program\",\"setup\":\"PageProgramSetup\"})]\nclass PageProgramClient\n  # @return [ProgramState]\n  #[GetRequest(\"/read\")]\n  def read\n  end\n\n  # @return [ProgramState]\n  #[PostRequest(\"/increment\")]\n  def increment\n  end\nend\n",
        ),
        (
            "client/setups/PageProgramSetup.rb",
            "#[Setup]\nclass PageProgramSetup\n  # @param execution [BehaviorExecution]\n  # @return [Hash]\n  def self.configure(execution)\n    $page_program_configured = ($page_program_configured || 0) + 1\n    identity = Object.new.object_id.to_s\n    count = 0\n    { http: { pageprogram: { exchange: ->(request, _execution) {\n      count += 1 if request[\"method\"] == \"POST\"\n      { \"identity\" => identity, \"count\" => count, \"configured\" => $page_program_configured }\n    } } } }\n  end\nend\n",
        ),
    ];

    const SERVICE: &'static str = "#[Service]\nclass __CLASS__\n  #[Needs]\n  # @return [PageProgramClient]\n  attr_reader :page_program_client\n\n  # @return [String]\n  attr_reader :identity\n  # @return [Integer]\n  attr_reader :count, :configured\n\n  def initialize\n    @identity = \"\"\n    @count = 0\n    @configured = 0\n  end\n\n  def look\n    show(page_program_client.read)\n  end\n\n  def increment\n    show(page_program_client.increment)\n  end\n\n  private\n\n  def show(state)\n    @identity = state.identity\n    @count = state.count\n    @configured = state.configured\n  end\nend\n";

    const DRIVER: &'static str = "const { default: WriterService } = await import('./WriterService.js')\n\
         const { default: ReaderService } = await import('./ReaderService.js')\n\
         const writer = new WriterService({}), reader = new ReaderService({})\n\
         await writer.increment()\n\
         await reader.look()\n\
         const seen = (service) => ({ identity: service.identity, count: service.count, configured: service.configured })\n\
         console.log(JSON.stringify({ writer: seen(writer), reader: seen(reader) }))\n\
         process.exit(0)\n";

    fn write_files(root: &Path) {
        for (relative, text) in Self::FILES {
            write(root, relative, text);
        }
    }

    fn service(class: &str) -> String {
        Self::SERVICE.replace("__CLASS__", class)
    }

    fn sources(root: &Path) -> Vec<(String, String, PathBuf)> {
        let writer = write(
            root,
            "client/services/WriterService.rb",
            &Self::service("WriterService"),
        );
        let reader = write(
            root,
            "client/services/ReaderService.rb",
            &Self::service("ReaderService"),
        );
        vec![
            (
                String::from("writer"),
                String::from("WriterService"),
                writer,
            ),
            (
                String::from("reader"),
                String::from("ReaderService"),
                reader,
            ),
        ]
    }
}

#[test]
fn two_ruby_services_share_one_prelude_and_program() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    PageProgramPage::write_files(root.path());
    let program = RubyWebServices::program(root.path(), &PageProgramPage::sources(root.path()))
        .unwrap_or_else(|failure| panic!("program: {failure}"));
    assert_eq!(program.matches(RubyWebServices::PRELUDE).count(), 1);
    assert_eq!(
        program
            .matches("ProgramState = Data.define(:identity, :count, :configured)")
            .count(),
        1,
        "{program}"
    );
    assert!(
        program.contains("TacServices.register(\"writer\", WriterService, '"),
        "{program}"
    );
    assert!(
        program.contains("TacServices.register(\"reader\", ReaderService, '"),
        "{program}"
    );
}

#[tokio::test]
async fn two_ruby_services_share_one_setup_and_client_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    PageProgramPage::write_files(root.path());
    let writer = PageProgramPage::service("WriterService");
    let reader = PageProgramPage::service("ReaderService");
    Page::publish(
        root.path(),
        &[
            ("writer", "WriterService", &writer),
            ("reader", "ReaderService", &reader),
        ],
    )
    .await;
    let report = Page::run(root.path(), PageProgramPage::DRIVER);
    let identity = report["writer"]["identity"].as_str().unwrap_or_default();
    assert!(!identity.is_empty(), "{report}");
    assert_eq!(
        report["writer"],
        json!({"identity": identity, "count": 1, "configured": 1}),
        "{report}"
    );
    assert_eq!(
        report["reader"],
        json!({"identity": identity, "count": 1, "configured": 1}),
        "{report}"
    );
}
