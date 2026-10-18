use super::{Page, write};
use crate::tac_ruby::{NeededServices, RubyWebServices};
use crate::tac_web::{BuildJobs, WebLanguage};
use std::path::Path;

struct NeededServicePage;

impl NeededServicePage {
    const COUNTER: &'static str = "#[Service]\nclass CounterService\n  attr_reader :count\n\n  def initialize\n    @count = 0\n  end\n\n  def increment\n    @count += 1\n  end\nend\n";
    const TALLY: &'static str = "#[Service]\nclass TallyService\n  #[Needs]\n  # @return [CounterService]\n  attr_reader :counter\n\n  def bump\n    counter.increment\n  end\nend\n";
    const STORED: &'static str = "#[Service]\nclass TallyService\n  #[Needs]\n  # @return [CounterStore]\n  attr_reader :counter_store\n\n  # @return [CounterService]\n  attr_reader :counter\nend\n";
    const MESSAGE: &'static str = "The `#[Needs]` field 'counter' names CounterService, another Tac service. A Ruby Tac service cannot need another service yet (ADR 0054): the page does not fill the field, so it would stay nil and the first call through it would fail.";

    const PLACE: &'static str = "#[Service]\nclass PlaceService\n  #[Needs]\n  # @return [Clipboard]\n  attr_reader :clipboard\n\n  # @return [Clock]\n  #[Needs]\n  attr_reader :clock\n\n  # @return [String]\n  def filled\n    \"#{clipboard.class.name} #{clock.class.name}\"\n  end\nend\n";
    const PLACE_DRIVER: &'static str = "const { default: PlaceService } = await import('./PlaceService.js')\n\
         const place = new PlaceService({})\n\
         console.log(JSON.stringify({ filled: place.filled(), keys: Object.keys(place) }))\n\
         process.exit(0)\n";
    const UNTYPED: &'static str = "#[Service]\nclass TallyService\n  #[Needs]\n  attr_reader :counter_store\n\n  # @return [Integer]\n  def total\n    counter_store.total\n  end\nend\n";
    const UNTYPED_MESSAGE: &'static str = "The `#[Needs]` field 'counter_store' has no type. A Ruby Tac service fills a Needs field from the `# @return [Type]` comment next to it (ADR 0054): without one the page does not fill the field, so it would stay nil and the first call through it would fail.";

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
            .is_some_and(|help| help
                .contains("a Store, a Converter, a Helper, an HTTPClient or a browser API")),
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

#[test]
fn a_ruby_needs_field_that_names_a_browser_api_is_filled_with_that_api() {
    let root = tempfile::tempdir().expect("project");
    let place = write(
        root.path(),
        "client/services/PlaceService.rb",
        NeededServicePage::PLACE,
    );
    let program = RubyWebServices::program(
        root.path(),
        &[(String::from("place"), String::from("PlaceService"), place)],
    )
    .unwrap_or_else(|failure| panic!("{failure}"));
    for expected in [
        "  private def clipboard\n    @clipboard ||= Clipboard.new\n  end",
        "  private def clock\n    @clock ||= Clock.new\n  end",
    ] {
        assert!(program.contains(expected), "{expected}: {program}");
    }
    assert!(!program.contains("attr_reader :clipboard"), "{program}");
    assert!(!program.contains("attr_reader :clock"), "{program}");
    let registration = program
        .lines()
        .find(|line| line.starts_with("TacServices.register(\"place\", PlaceService, '"))
        .expect("registration");
    assert!(registration.contains("\"fields\":[]"), "{registration}");
}

#[tokio::test]
async fn a_ruby_needs_field_that_names_a_browser_api_is_not_nil_on_the_page() {
    if let Some(reason) = Page::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    Page::publish(
        root.path(),
        &[("place", "PlaceService", NeededServicePage::PLACE)],
    )
    .await;
    let report = Page::run(root.path(), NeededServicePage::PLACE_DRIVER);
    assert_eq!(report["filled"], "Clipboard Clock", "{report}");
    assert_eq!(report["keys"], serde_json::json!([]), "{report}");
}

#[tokio::test]
async fn a_ruby_needs_field_without_a_type_stops_the_build_at_the_field() {
    let root = tempfile::tempdir().expect("project");
    let tally = write(
        root.path(),
        "client/services/TallyService.rb",
        NeededServicePage::UNTYPED,
    );
    let chosen = [(
        String::from("tally"),
        String::from("TallyService"),
        WebLanguage::Ruby,
        tally,
    )];
    let jobs = BuildJobs::new(1);
    let built =
        crate::tac_web::build_all(root.path(), &chosen, &jobs, crate::BuildProfile::Release).await;
    let Err(failure) = built else {
        panic!("the build accepted a Ruby Needs field without a type");
    };
    assert_eq!(failure.diagnostics().len(), 1, "{failure}");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.to_string(), "TY2015", "{failure}");
    assert_eq!(
        diagnostic.message,
        NeededServicePage::UNTYPED_MESSAGE,
        "{failure}"
    );
    assert!(
        diagnostic
            .help
            .as_deref()
            .is_some_and(|help| help.contains("`# @return [CounterStore]`")),
        "{failure}"
    );
    let span = &diagnostic.spans[0];
    assert_eq!(span.file, "client/services/TallyService.rb", "{failure}");
    let start = usize::try_from(span.start).expect("start");
    let end = usize::try_from(span.end).expect("end");
    assert_eq!(
        &NeededServicePage::UNTYPED[start..end],
        "  attr_reader :counter_store"
    );
}
