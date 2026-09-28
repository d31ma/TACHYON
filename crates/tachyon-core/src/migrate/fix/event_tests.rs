use super::MigrationFix;
use super::visibility_off::VisibilityOff;
use std::fs;
use std::path::Path;

struct Samples;

impl Samples {
    const TYPESCRIPT_MODEL: &str = "export interface OrderPlaced {\n  id: string\n}\n";
    const TYPESCRIPT_SERVICE: &str = "export default class OrdersService {\n  applicationEventPublisher!: ApplicationEventPublisher\n  place(order: any): void {\n    this.applicationEventPublisher.publishEvent('order.placed', order, true)\n  }\n  @OnEvent({\"event\":\"order.placed\",\"replay\":true})\n  placed(order: any): void {}\n}\n";
    const TYPESCRIPT_EXPECTED: &str = "export default class OrdersService {\n  applicationEventPublisher!: ApplicationEventPublisher\n  place(order: any): void {\n    this.applicationEventPublisher.publishEvent(order satisfies OrderPlaced, true)\n  }\n  @OnEvent({\"replay\":true})\n  placed(order: OrderPlaced): void {}\n}\n";
    const JAVASCRIPT_MODEL: &str =
        "/**\n * @typedef {Object} OrderPlaced\n * @property {string} id\n */\nexport {}\n";
    const JAVASCRIPT_SERVICE: &str = "export default class OrdersService {\n  applicationEventPublisher;\n  place(order) { this.applicationEventPublisher.publishEvent('OrderPlaced', order); }\n  @Order(2)\n  @OnEvent({\"event\":\"OrderPlaced\"})\n  placed(order) {}\n}\n";
    const JAVASCRIPT_EXPECTED: &str = "export default class OrdersService {\n  applicationEventPublisher;\n  place(order) { this.applicationEventPublisher.publishEvent(/** @type {OrderPlaced} */ (order)); }\n  @Order(2)\n  /** @param {OrderPlaced} order */\n  @OnEvent\n  placed(order) {}\n}\n";
    const JAVA_MODEL: &str = "record OrderPlaced(String id) {}\n";
    const JAVA_SERVICE: &str = "@Service\nclass OrdersService {\n  ApplicationEventPublisher publisher;\n  void place(OrderPlaced order) { publisher.publishEvent(\"order.placed\", order); }\n  @OnEvent({\"event\":\"order.placed\"})\n  void placed(OrderPlaced order) {}\n  @OnEvent({\"event\":\"order.shipped\"})\n  void shipped(Object order) {}\n}\n";
    const JAVA_EXPECTED: &str = "@Service\nclass OrdersService {\n  ApplicationEventPublisher publisher;\n  void place(OrderPlaced order) { publisher.publishEvent(\"order.placed\", order); }\n  @OnEvent\n  void placed(OrderPlaced order) {}\n  @OnEvent({\"event\":\"order.shipped\"})\n  void shipped(Object order) {}\n}\n";
}

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(path, source).expect("source");
}

fn converted(root: &Path, relative: &str) -> String {
    VisibilityOff::write(root);
    let planned = MigrationFix::plan(root).expect("plan");
    planned.write(root).expect("write");
    let again = MigrationFix::plan(root).expect("second plan");
    assert_eq!(
        again.change_count(),
        0,
        "a second run changes {relative} again"
    );
    fs::read_to_string(root.join(relative)).expect("converted source")
}

#[test]
fn javascript_and_typescript_events_become_models() {
    let project = tempfile::tempdir().expect("project");
    write(
        project.path(),
        "client/models/Orders.ts",
        Samples::TYPESCRIPT_MODEL,
    );
    write(
        project.path(),
        "client/services/OrdersService.ts",
        Samples::TYPESCRIPT_SERVICE,
    );
    let written = converted(project.path(), "client/services/OrdersService.ts");
    assert_eq!(written, Samples::TYPESCRIPT_EXPECTED);
    let project = tempfile::tempdir().expect("project");
    write(
        project.path(),
        "client/models/OrderPlaced.js",
        Samples::JAVASCRIPT_MODEL,
    );
    write(
        project.path(),
        "client/services/OrdersService.js",
        Samples::JAVASCRIPT_SERVICE,
    );
    let written = converted(project.path(), "client/services/OrdersService.js");
    assert_eq!(written, Samples::JAVASCRIPT_EXPECTED);
}

#[test]
fn typed_languages_convert_listeners_and_report_what_needs_a_person() {
    let project = tempfile::tempdir().expect("project");
    VisibilityOff::write(project.path());
    write(
        project.path(),
        "server/models/OrderPlaced.java",
        Samples::JAVA_MODEL,
    );
    write(
        project.path(),
        "server/services/OrdersService.java",
        Samples::JAVA_SERVICE,
    );
    let planned = MigrationFix::plan(project.path()).expect("plan");
    let mut problems = Vec::new();
    for step in &planned.manual_steps {
        problems.push(step.problem.clone());
    }
    let joined = problems.join("\n");
    assert!(
        joined.contains(
            "publishEvent names the event 'order.placed'; it now takes a OrderPlaced value."
        ),
        "{joined}"
    );
    assert!(
        joined.contains("The event name 'order.shipped' has no model with the same name."),
        "{joined}"
    );
    planned.write(project.path()).expect("write");
    let written = fs::read_to_string(project.path().join("server/services/OrdersService.java"))
        .expect("source");
    assert_eq!(written, Samples::JAVA_EXPECTED);
}

#[test]
fn ruby_listeners_with_a_yard_model_type_lose_their_event_names() {
    let project = tempfile::tempdir().expect("project");
    write(
        project.path(),
        "client/models/OrderPlaced.rb",
        "# @!attribute [r] id\n#   @return [String]\nOrderPlaced = Data.define(:id)\n",
    );
    write(
        project.path(),
        "client/services/OrdersService.rb",
        "#[Service]\nclass OrdersService\n  def place(order)\n    @events.publish_event(\"order.placed\", order)\n  end\n\n  # @param order [OrderPlaced]\n  #[OnEvent({\"event\":\"order.placed\",\"replay\":true})]\n  #[Order(2)]\n  def placed(order)\n  end\n\n  #[OnEvent({\"event\":\"OrderPlaced\"})]\n  def untyped(order)\n  end\nend\n",
    );
    let planned = MigrationFix::plan(project.path()).expect("plan");
    let problems: Vec<String> = planned
        .manual_steps
        .iter()
        .map(|step| step.problem.clone())
        .collect();
    let joined = problems.join("\n");
    assert!(
        joined.contains(
            "publish_event names the event 'order.placed'; it now takes a OrderPlaced value."
        ),
        "{joined}"
    );
    assert!(
        joined.contains("The OnEvent listener for 'OrderPlaced' reads a JSON value"),
        "{joined}"
    );
    planned.write(project.path()).expect("write");
    let written = fs::read_to_string(project.path().join("client/services/OrdersService.rb"))
        .expect("source");
    assert!(
        written.contains("  # @param order [OrderPlaced]\n  #[OnEvent({\"replay\":true})]\n  #[Order(2)]\n  def placed(order)"),
        "{written}"
    );
    assert!(
        written.contains("  #[OnEvent({\"event\":\"OrderPlaced\"})]\n  def untyped(order)"),
        "{written}"
    );
}
