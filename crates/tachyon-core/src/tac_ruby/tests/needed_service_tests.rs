use super::write;
use crate::tac_ruby::NeededServices;
use crate::tac_web::{BuildJobs, WebLanguage};
use std::path::Path;

struct NeededServicePage;

impl NeededServicePage {
    const COUNTER: &'static str = "#[Service]\nclass CounterService\n  attr_reader :count\n\n  def initialize\n    @count = 0\n  end\n\n  def increment\n    @count += 1\n  end\nend\n";
    const TALLY: &'static str = "#[Service]\nclass TallyService\n  #[Needs]\n  # @return [CounterService]\n  attr_reader :counter\n\n  def bump\n    counter.increment\n  end\nend\n";
    const STORED: &'static str = "#[Service]\nclass TallyService\n  #[Needs]\n  # @return [CounterStore]\n  attr_reader :counter_store\n\n  # @return [CounterService]\n  attr_reader :counter\nend\n";
    const MESSAGE: &'static str = "The `#[Needs]` field 'counter' names CounterService, another Tac service. A Ruby Tac service cannot need another service yet (ADR 0054): the page does not fill the field, so it would stay nil and the first call through it would fail.";

    fn chosen(root: &Path) -> Vec<(String, String, WebLanguage, std::path::PathBuf)> {
        let counter = write(root, "client/services/CounterService.rb", Self::COUNTER);
        let tally = write(root, "client/services/TallyService.rb", Self::TALLY);
        vec![
            (
                String::from("counter"),
                String::from("CounterService"),
                WebLanguage::Ruby,
                counter,
            ),
            (
                String::from("tally"),
                String::from("TallyService"),
                WebLanguage::Ruby,
                tally,
            ),
        ]
    }
}

#[tokio::test]
async fn a_ruby_service_that_needs_another_service_stops_the_build_at_the_field() {
    let root = tempfile::tempdir().expect("project");
    let chosen = NeededServicePage::chosen(root.path());
    let jobs = BuildJobs::new(1);
    let built =
        crate::tac_web::build_all(root.path(), &chosen, &jobs, crate::BuildProfile::Release).await;
    let Err(failure) = built else {
        panic!("the build accepted a Ruby service that needs another service");
    };
    assert_eq!(failure.diagnostics().len(), 1, "{failure}");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.to_string(), "TY2015", "{failure}");
    assert_eq!(diagnostic.message, NeededServicePage::MESSAGE, "{failure}");
    assert!(
        diagnostic
            .help
            .as_deref()
            .is_some_and(|help| help.contains("a Store, a Converter or an HTTPClient")),
        "{failure}"
    );
    let span = &diagnostic.spans[0];
    assert_eq!(span.file, "client/services/TallyService.rb", "{failure}");
    let start = usize::try_from(span.start).expect("start");
    let end = usize::try_from(span.end).expect("end");
    assert_eq!(
        &NeededServicePage::TALLY[start..end],
        "  attr_reader :counter"
    );
}

#[test]
fn a_ruby_service_still_needs_a_store_and_keeps_an_unmarked_service_field() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/services/CounterService.rb",
        NeededServicePage::COUNTER,
    );
    let checked = NeededServices::RUBY.refuse(
        root.path(),
        Path::new("client/services/TallyService.rb"),
        NeededServicePage::STORED,
    );
    assert!(checked.is_ok(), "{checked:?}");
}
