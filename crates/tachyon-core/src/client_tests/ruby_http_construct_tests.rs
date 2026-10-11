#![allow(clippy::expect_used)]

use std::path::Path;

struct RubyHttpConstructProject;

impl RubyHttpConstructProject {
    const FILES: [(&'static str, &'static str); 8] = [
        (
            "client/models/Greeting.rb",
            "# @!attribute [r] message\n#   @return [String]\nGreeting = Data.define(:message)\n",
        ),
        (
            "client/remotes/GreetingClient.rb",
            "#[HTTPClient(\"/api\")]\nclass GreetingClient\n  # @return [Greeting]\n  #[GetRequest(\"/greeting\")]\n  def read\n  end\n\n  # @return [Array<Greeting>]\n  #[GetRequest(\"/greetings\")]\n  def all\n  end\n\n  # @return [Greeting]\n  #[GetRequest(\"/wrong\")]\n  def wrong\n  end\nend\n",
        ),
        (
            "client/remotes/OverrideClient.rb",
            "#[HTTPClient(\"/api\")]\nclass OverrideClient\n  # @return [Greeting]\n  #[GetRequest(\"/never\")]\n  def read\n  end\nend\n",
        ),
        (
            "client/setups/GreetingSetup.rb",
            "#[Setup]\nclass GreetingSetup\n  # @param execution [BehaviorExecution]\n  # @return [Hash]\n  def self.configure(execution)\n    {}\n  end\nend\n",
        ),
        (
            "client/setups/OverrideSetup.rb",
            "#[Setup]\nclass OverrideSetup\n  # @param execution [BehaviorExecution]\n  # @return [Hash]\n  def self.configure(execution)\n    { http: { override: { exchange: ->(_request, _execution) { { \"message\" => \"Custom adapter wins\" } } } } }\n  end\nend\n",
        ),
        (
            "client/services/GreetingService.rb",
            "#[Service]\nclass GreetingService\n  #[Needs]\n  # @return [GreetingClient]\n  attr_reader :greeting_client\n\n  #[Needs]\n  # @return [OverrideClient]\n  attr_reader :override_client\n\n  # @return [String]\n  attr_reader :message\n\n  def initialize\n    @message = \"\"\n  end\n\n  # @return [void]\n  def ask\n    @message = greeting_client.read.message\n  end\n\n  # @return [Array<String>]\n  def names\n    greeting_client.all.map(&:message)\n  end\n\n  # @return [Greeting]\n  def wrong\n    greeting_client.wrong\n  end\n\n  # @return [String]\n  def custom\n    override_client.read.message\n  end\nend\n",
        ),
        (
            "client/tests/GreetingTestSetup.rb",
            "#[TestSetup]\nclass GreetingTestSetup\n  # @param execution [BehaviorExecution]\n  # @return [Hash]\n  def self.configure(execution)\n    { http: { greeting: { exchange: method(:exchange) } } }\n  end\n\n  # @param request [Hash]\n  # @param execution [BehaviorExecution]\n  # @return [Object]\n  def self.exchange(request, execution)\n    return [{ \"message\" => \"one\" }, { \"message\" => \"two\" }] if request[\"path\"] == \"/api/greetings\"\n    return \"not a greeting\" if request[\"path\"] == \"/api/wrong\"\n    { \"message\" => \"Test adapter wins\" }\n  end\nend\n",
        ),
        (
            "client/tests/GreetingServiceTest.rb",
            "#[Test(GreetingTestSetup, {\"partition\":\"ruby-http-tests\"})]\nclass GreetingServiceTest\n  #[Fake]\n  attr_accessor :greeting_client\n\n  # @return [void]\n  def test_an_empty_fake_uses_the_test_adapter\n    service = construct(GreetingService)\n    service.ask\n    Assert.equals(\"Test adapter wins\", service.message)\n  end\n\n  # @return [void]\n  def test_list_answers_are_typed\n    Assert.equals([\"one\", \"two\"], construct(GreetingService).names)\n  end\n\n  # @return [void]\n  def test_an_assigned_fake_replaces_the_client\n    self.greeting_client = Struct.new(:read).new(Greeting.new(message: \"Fake wins\"))\n    service = construct(GreetingService)\n    service.ask\n    Assert.equals(\"Fake wins\", service.message)\n  end\n\n  # @return [void]\n  def test_a_wrong_answer_shape_fails_with_http_response\n    code = \"\"\n    begin\n      construct(GreetingService).wrong\n    rescue BehaviorError => error\n      code = error.behavior_code\n    end\n    Assert.equals(\"HTTP_RESPONSE\", code)\n  end\n\n  # @return [void]\n  def test_a_client_without_an_adapter_keeps_its_failure\n    message = \"\"\n    begin\n      construct(GreetingService).custom\n    rescue BehaviorError => error\n      message = error.message\n    end\n    Assert.equals(\"Required behavior adapter is missing.\", message)\n  end\nend\n",
        ),
    ];

    fn write(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }
}

#[test]
fn ruby_tests_construct_services_with_injected_fake_clients_and_typed_answers() {
    let ruby_installed = std::process::Command::new("ruby")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !ruby_installed {
        assert!(std::env::var_os("CI").is_none(), "CI must provide ruby");
        eprintln!("skipping: ruby is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    RubyHttpConstructProject::write(root.path());
    let report = super::run(
        root.path(),
        Path::new("client/tests/GreetingServiceTest.rb"),
    )
    .expect("Ruby test unit");
    assert_eq!(report.cases.len(), 5, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
