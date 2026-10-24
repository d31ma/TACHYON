#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct Layout;

impl Layout {
    const ROLES: [(&'static str, &'static str); 16] = [
        ("@RequestRoute", "controllers"),
        ("[<RequestRoute", "controllers"),
        ("[<Service", "services"),
        ("@request_route", "controllers"),
        ("#[RequestRoute", "controllers"),
        ("[RequestRoute", "controllers"),
        ("@Service", "services"),
        ("@service", "services"),
        ("#[Service", "services"),
        ("[Service", "services"),
        ("#[yon::service", "services"),
        ("@Store", "stores"),
        ("@store", "stores"),
        ("#[Store", "stores"),
        ("[Store", "stores"),
        ("#[yon::store", "stores"),
    ];

    fn folder(line: &str) -> Option<&'static str> {
        let line = line.strip_prefix("// ").unwrap_or(line);
        if line.to_ascii_lowercase().starts_with("@yon::service") {
            return Some("services");
        }
        if line.to_ascii_lowercase().starts_with("@yon::store") {
            return Some("stores");
        }
        Self::ROLES
            .iter()
            .find(|(marker, _)| line.starts_with(marker))
            .map(|(_, folder)| *folder)
    }

    fn blocks(source: &str) -> Vec<(usize, &'static str)> {
        let mut offset = 0;
        let mut blocks = Vec::new();
        let mut imports = None;
        for line in source.split_inclusive('\n') {
            if line.starts_with("import ") {
                imports.get_or_insert(offset);
            } else if let Some(folder) = Self::folder(line) {
                blocks.push((imports.unwrap_or(offset), folder));
                imports = None;
            } else if !line.starts_with("#[") && !line.starts_with("// @") {
                imports = None;
            }
            offset += line.len();
        }
        blocks
    }

    fn class_name(block: &str) -> &str {
        for line in block.lines() {
            let declaration = line
                .strip_prefix("class ")
                .or_else(|| line.strip_prefix("struct "))
                .or_else(|| line.strip_prefix("type "));
            if let Some(declaration) = declaration {
                return declaration
                    .split(|character: char| !character.is_ascii_alphanumeric())
                    .next()
                    .expect("class name");
            }
        }
        panic!("fixture block declares a class");
    }

    fn write(root: &Path, extension: &str, contents: &str) -> String {
        let source = contents.trim_end().trim_end_matches("?>");
        let source = source.trim_start().strip_prefix("<?php").unwrap_or(source);
        let blocks = Self::blocks(source);
        let mut controller = String::new();
        for (index, &(start, folder)) in blocks.iter().enumerate() {
            let end = blocks.get(index + 1).map_or(source.len(), |next| next.0);
            let block = &source[start..end];
            let relative = format!("server/{folder}/{}.{extension}", Self::class_name(block));
            let text = if extension == "php" {
                format!("<?php\n{block}")
            } else {
                block.to_owned()
            };
            let path = root.join(&relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("directories");
            fs::write(path, text).expect("source");
            if folder == "controllers" {
                controller = relative;
            }
        }
        assert!(!controller.is_empty(), "the fixture declares a controller");
        controller
    }
}

fn available(program: &str) -> bool {
    let found = Command::new(program)
        .arg(match program {
            "kotlinc" => "-version",
            "go" => "version",
            _ => "--version",
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    crate::support::toolchain::Toolchain::checked(program, found)
}

fn explicit_controller_fields(source: &str, extension: &str) -> String {
    let (marker, replacement) = match extension {
        "js" => (
            "@NeedsAll()\nclass MoneyController",
            "@NeedsAll(\"rightService\", \"leftService\")\nclass MoneyController",
        ),
        "py" => (
            "@needs_all()\nclass MoneyController",
            "@needs_all(\"right\", \"left\")\nclass MoneyController",
        ),
        "php" | "rs" => ("#[NeedsAll()]\n", "#[NeedsAll(\"right\", \"left\")]\n"),
        "go" => ("// @NeedsAll()\n", "// @NeedsAll(\"right\", \"left\")\n"),
        "cs" => (
            "[NeedsAll()]\nclass MoneyController",
            "[NeedsAll(\"right\", \"left\")]\nclass MoneyController",
        ),
        _ => (
            "@NeedsAll()\nclass MoneyController",
            "@NeedsAll(\"right\", \"left\")\nclass MoneyController",
        ),
    };
    let at = source.rfind(marker).expect("controller constructor marker");
    let mut result = source.to_owned();
    result.replace_range(at..at + marker.len(), replacement);
    result
}

async fn assert_inferred_graph(extension: &str, program: &str, fixture: &str) {
    if !available(program) {
        eprintln!("Skipping .{extension} constructor inference: {program} unavailable");
        return;
    }
    let lowercase = fixture.to_ascii_lowercase().replace("needs()", "");
    assert!(!lowercase.contains("needs("));
    assert_field_selection(extension, fixture, false).await;
    let listed = explicit_controller_fields(fixture, extension);
    let root = tempfile::tempdir().expect("project");
    let relative = Layout::write(root.path(), extension, &listed);
    let failure = HandlerSource::discover(root.path(), &relative)
        .expect_err("a NeedsAll field list is refused")
        .to_string();
    assert!(
        failure.contains("NeedsAll takes no field list"),
        "{extension}: {failure}"
    );
}

async fn assert_field_selection(extension: &str, fixture: &str, explicit: bool) {
    let contents = if explicit {
        explicit_controller_fields(fixture, extension)
    } else {
        fixture.to_owned()
    };
    assert_scope_answers(extension, &contents, &format!("explicit={explicit}"), true).await;
}

async fn assert_lifetime_effect(extension: &str, program: &str, fixture: &str, marker: &str) {
    if !available(program) {
        eprintln!("Skipping .{extension} Lifetime: {program} unavailable");
        return;
    }
    assert!(
        fixture.contains(marker),
        "{extension}: the fixture marks {marker}"
    );
    assert_scope_answers(extension, fixture, "transient", true).await;
    let shared = fixture.replace(marker, "");
    assert_scope_answers(extension, &shared, "without Lifetime", false).await;
}

async fn assert_scope_answers(extension: &str, contents: &str, label: &str, fresh: bool) {
    let root = tempfile::tempdir().expect("project");
    let relative = Layout::write(root.path(), extension, contents);
    let source = HandlerSource::discover(root.path(), &relative)
        .unwrap_or_else(|failure| panic!("{extension}, {label}: discovery: {failure}"));
    let mut options = HandlerSupervisorOptions {
        default_timeout: Duration::from_mins(2),
        ..HandlerSupervisorOptions::default()
    };
    if extension == "ts" {
        options.runtimes.javascript = "bun".into();
    }
    let supervisor = HandlerSupervisor::new(options).expect("bounded supervisor");
    assert_repeated_invocations(extension, label, fresh, &source, &supervisor).await;
}

async fn assert_repeated_invocations(
    extension: &str,
    label: &str,
    fresh: bool,
    source: &HandlerSource,
    supervisor: &HandlerSupervisor,
) {
    for invocation in 0..2 {
        let request = HandlerRequest::route(
            format!("inferred_{extension}_{fresh}_{invocation}"),
            "/money",
            HttpMethod::Get,
        );
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .expect("inferred controller response");
        assert!(
            response.error.is_none(),
            "{extension}, {label}: {:?}",
            response.error
        );
        assert_eq!(response.status, 200, "{extension}, {label}");
        let body: serde_json::Value =
            serde_json::from_str(&response.body.expect("JSON response").data)
                .expect("response JSON");
        assert_eq!(
            body,
            serde_json::json!({"shared":true,"fresh":fresh,"order":"left:right","amount":42}),
            "{extension}, {label}, invocation={invocation}"
        );
    }
}

struct Fixtures;

impl Fixtures {
    const JAVASCRIPT: &str = r#"
@Service()
@NeedsAll()
class SharedService {
  static ignored;
  value = 42;
  amount() { return this.value; }
}
@Service()
@Lifetime("transient")
@NeedsAll()
class FreshService {}
@Service()
@NeedsAll()
class LeftService {
  sharedService;
  freshService;
  side() { return "left"; }
}
@Service()
@NeedsAll()
class RightService {
  sharedService;
  freshService;
  side() { return "right"; }
}
@RequestRoute("/money")
@Public
@NeedsAll()
class MoneyController {
  leftService;
  rightService;
  @GetRoute
  getMoney() {
    return {shared:this.leftService.sharedService === this.rightService.sharedService,
      fresh:this.leftService.freshService !== this.rightService.freshService,
      order:this.leftService.side() + ":" + this.rightService.side(),
      amount:this.leftService.sharedService.amount()};
  }
}
"#;

    const TYPESCRIPT: &str = r#"
@Service()
@NeedsAll()
class SharedService {
  static ignored: string;
  value: number = 42;
  amount() { return this.value; }
}
@Service()
@Lifetime("transient")
@NeedsAll()
class FreshService {}
@Service()
@NeedsAll()
class LeftService {
  shared!: SharedService;
  fresh!: FreshService;
  side() { return "left"; }
}
@Service()
@NeedsAll()
class RightService {
  shared: SharedService;
  fresh: FreshService;
  side() { return "right"; }
}
@RequestRoute("/money")
@Public
@NeedsAll()
class MoneyController {
  left: LeftService;
  right: RightService;
  @GetRoute
  getMoney() {
    return {shared:this.left.shared === this.right.shared,
      fresh:this.left.fresh !== this.right.fresh,
      order:this.left.side() + ":" + this.right.side(),
      amount:this.left.shared.amount()};
  }
}
"#;

    const PYTHON: &str = r#"
@service()
@needs_all()
class SharedService:
    def amount(self): return 42
@service()
@lifetime("transient")
@needs_all()
class FreshService:
    pass
@service()
@needs_all()
class LeftService:
    shared: SharedService
    fresh: FreshService
    def side(self): return "left"
@service()
@needs_all()
class RightService:
    shared: SharedService
    fresh: FreshService
    def side(self): return "right"
@request_route("/money")
@public
@needs_all()
class MoneyController:
    left: LeftService
    right: RightService
    @get_route
    def get_money(self):
        return {"shared":self.left.shared is self.right.shared,
                "fresh":self.left.fresh is not self.right.fresh,
                "order":self.left.side() + ":" + self.right.side(),
                "amount":self.left.shared.amount()}
"#;

    const PHP: &str = r#"<?php
#[Service]
#[NeedsAll()]
class SharedService { public function amount() { return 42; } }
#[Service]
#[Lifetime("transient")]
#[NeedsAll()]
class FreshService {}
#[Service]
#[NeedsAll()]
class LeftService {
    public SharedService $shared;
    public FreshService $fresh;
    public function side() { return "left"; }
}
#[Service]
#[NeedsAll()]
class RightService {
    public SharedService $shared;
    public FreshService $fresh;
    public function side() { return "right"; }
}
#[RequestRoute("/money")]
#[Public]
#[NeedsAll()]
class MoneyController {
    public LeftService $left;
    public RightService $right;
    #[GetRoute]
    public function getMoney() {
        return [
            "shared"=>$this->left->shared === $this->right->shared,
            "fresh"=>$this->left->fresh !== $this->right->fresh,
            "order"=>$this->left->side() . ":" . $this->right->side(),
            "amount"=>$this->left->shared->amount()
        ];
    }
}
"#;

    const JAVA: &str = r#"
@Service
@NeedsAll()
class SharedService { int amount() { return 42; } }
@Service
@Lifetime("transient")
@NeedsAll()
class FreshService {}
@Service
@NeedsAll()
class LeftService {
    final SharedService shared;
    final FreshService fresh;
    String side() { return "left"; }
}
@Service
@NeedsAll()
class RightService {
    final SharedService shared;
    final FreshService fresh;
    String side() { return "right"; }
}
@RequestRoute("/money")
@Public
@NeedsAll()
class MoneyController {
    final LeftService left;
    final RightService right;
    @GetRoute
    Object getMoney() {
        return java.util.Map.of(
            "shared",left.shared == right.shared,
            "fresh",left.fresh != right.fresh,
            "order",left.side() + ":" + right.side(),
            "amount",left.shared.amount());
    }
}
"#;

    const GROOVY: &str = r#"
@Service
@NeedsAll()
class SharedService {
    int amount() { return 42 }
}
@Service
@Lifetime('transient')
@NeedsAll()
class FreshService {}
@Service
@NeedsAll()
class LeftService {
    final SharedService shared
    final FreshService fresh
    String side() { return 'left' }
}
@Service
@NeedsAll()
class RightService {
    final SharedService shared
    final FreshService fresh
    String side() { return 'right' }
}
@RequestRoute('/money')
@Public
@NeedsAll()
class MoneyController {
    final LeftService left
    final RightService right
    @GetRoute
    Object getMoney() {
        return [
            shared: left.shared.is(right.shared),
            fresh: !left.fresh.is(right.fresh),
            order: "${left.side()}:${right.side()}".toString(),
            amount: left.shared.amount()
        ]
    }
}
"#;

    const KOTLIN: &str = r#"
@Service
@NeedsAll()
class SharedService { fun amount(): Int = 42 }
@Service
@Lifetime("transient")
@NeedsAll()
class FreshService {}
@Service
@NeedsAll()
class LeftService {
    val shared: SharedService
    val fresh: FreshService
    fun side(): String = "left"
}
@Service
@NeedsAll()
class RightService {
    lateinit var shared: SharedService
    lateinit var fresh: FreshService
    fun side(): String = "right"
}
@RequestRoute("/money")
@Public
@NeedsAll()
class MoneyController {
    val left: LeftService
    val right: RightService
    @GetRoute
    fun getMoney(): Any? = mapOf(
        "shared" to (left.shared === right.shared),
        "fresh" to (left.fresh !== right.fresh),
        "order" to (left.side() + ":" + right.side()),
        "amount" to left.shared.amount())
}
"#;

    const CSHARP: &str = r#"
[Service]
[NeedsAll()]
class SharedService { public int amount() { return 42; } }
[Service]
[Lifetime("transient")]
[NeedsAll()]
class FreshService {}
[Service]
[NeedsAll()]
class LeftService {
    internal readonly SharedService _shared;
    internal readonly FreshService _fresh;
    public string side() { return "left"; }
}
[Service]
[NeedsAll()]
class RightService {
    internal readonly SharedService _shared;
    internal readonly FreshService _fresh;
    public string side() { return "right"; }
}
[RequestRoute("/money")]
[Public]
[NeedsAll()]
class MoneyController {
    readonly LeftService _left;
    readonly RightService _right;
    [GetRoute]
    public object GetMoney() {
        return new {
            shared = object.ReferenceEquals(_left._shared, _right._shared),
            fresh = !object.ReferenceEquals(_left._fresh, _right._fresh),
            order = _left.side() + ":" + _right.side(),
            amount = _left._shared.amount()
        };
    }
}
"#;

    const RUST: &str = r#"
#[yon::service]
#[NeedsAll()]
struct SharedService {}
impl SharedService { fn amount(&self) -> i32 { 42 } }
#[yon::service]
#[yon::lifetime("transient")]
#[NeedsAll()]
struct FreshService {}
#[yon::service]
#[NeedsAll()]
struct LeftService {
    shared: std::sync::Arc<SharedService>,
    fresh: std::sync::Arc<FreshService>,
}
impl LeftService { fn side(&self) -> &'static str { "left" } }
#[yon::service]
#[NeedsAll()]
struct RightService {
    shared: std::sync::Arc<SharedService>,
    fresh: std::sync::Arc<FreshService>,
}
impl RightService { fn side(&self) -> &'static str { "right" } }
#[RequestRoute("/money")]
#[Public]
#[NeedsAll()]
struct MoneyController {
    left: std::sync::Arc<LeftService>,
    right: std::sync::Arc<RightService>,
}
impl MoneyController {
    #[GetRoute]
    fn get_money(&self) -> Json {
        Json::map([
            ("shared", Json::from(std::sync::Arc::ptr_eq(&self.left.shared, &self.right.shared))),
            ("fresh", Json::from(!std::sync::Arc::ptr_eq(&self.left.fresh, &self.right.fresh))),
            ("order", Json::from(format!("{}:{}", self.left.side(), self.right.side()))),
            ("amount", Json::from(self.left.shared.amount())),
        ])
    }
}
"#;

    const GO: &str = r#"
// @yon::Service
// @NeedsAll()
type SharedService struct{}

func (service *SharedService) Amount() int { return 42 }

// @yon::Service
// @yon::Lifetime("transient")
type FreshService struct {
	serial int
}
// @yon::Service
// @NeedsAll()
type LeftService struct {
	shared *SharedService
	fresh  *FreshService
}

func (service *LeftService) Side() string { return "left" }

// @yon::Service
// @NeedsAll()
type RightService struct {
	shared *SharedService
	fresh  *FreshService
}

func (service *RightService) Side() string { return "right" }

// @RequestRoute("/money")
// @Public
// @NeedsAll()
type MoneyController struct {
	left  *LeftService
	right *RightService
}

// @GetRoute
func (controller *MoneyController) GetMoney() any {
	return map[string]any{
		"shared": controller.left.shared == controller.right.shared,
		"fresh":  controller.left.fresh != controller.right.fresh,
		"order":  controller.left.Side() + ":" + controller.right.Side(),
		"amount": controller.left.shared.Amount(),
	}
}
"#;
    const DART: &str = r#"
@Service
class SharedService {
  int amount() => 42;
}
@Service
@Lifetime("transient")
class FreshService {}
@Service
class LeftService {
  @Needs
  final SharedService shared;
  @Needs
  final FreshService fresh;
  String side() => 'left';
}
@Service
class RightService {
  @Needs
  final SharedService shared;
  @Needs
  final FreshService fresh;
  String side() => 'right';
}
@RequestRoute("/money")
@Public
class MoneyController {
  @Needs
  final LeftService left;
  @Needs
  final RightService right;

  @GetRoute
  Map<String, Object?> getMoney() => {
        'shared': identical(left.shared, right.shared),
        'fresh': !identical(left.fresh, right.fresh),
        'order': '${left.side()}:${right.side()}',
        'amount': left.shared.amount(),
      };
}
"#;

    const FSHARP: &str = r#"
[<Service>]
type SharedService() =
    member this.Amount() : int = 42

[<Service>]
[<Lifetime("transient")>]
type FreshService() =
    member this.Serial : int = 0

[<Service>]
type LeftService() =
    [<Needs>]
    member val Shared : SharedService = Unchecked.defaultof<SharedService> with get, set
    [<Needs>]
    member val Fresh : FreshService = Unchecked.defaultof<FreshService> with get, set
    member this.Side() : string = "left"

[<Service>]
type RightService() =
    [<Needs>]
    member val Shared : SharedService = Unchecked.defaultof<SharedService> with get, set
    [<Needs>]
    member val Fresh : FreshService = Unchecked.defaultof<FreshService> with get, set
    member this.Side() : string = "right"

[<RequestRoute("/money")>]
[<Public>]
type MoneyController() =
    [<Needs>]
    member val Left : LeftService = Unchecked.defaultof<LeftService> with get, set
    [<Needs>]
    member val Right : RightService = Unchecked.defaultof<RightService> with get, set

    [<GetRoute>]
    member this.GetMoney() : obj =
        dict [
            "shared", box (obj.ReferenceEquals(this.Left.Shared, this.Right.Shared))
            "fresh", box (not (obj.ReferenceEquals(this.Left.Fresh, this.Right.Fresh)))
            "order", box (this.Left.Side() + ":" + this.Right.Side())
            "amount", box (this.Left.Shared.Amount())
        ]
        |> box
"#;
}

#[tokio::test]
async fn javascript_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("js", "node", Fixtures::JAVASCRIPT).await;
}

#[tokio::test]
async fn typescript_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("ts", "bun", Fixtures::TYPESCRIPT).await;
}

#[tokio::test]
async fn python_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("py", "python3", Fixtures::PYTHON).await;
}

#[tokio::test]
async fn php_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("php", "php-cgi", Fixtures::PHP).await;
}

#[tokio::test]
async fn java_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("java", "java", Fixtures::JAVA).await;
}

#[tokio::test]
async fn groovy_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("groovy", "groovyc", Fixtures::GROOVY).await;
}

#[tokio::test]
async fn kotlin_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("kt", "kotlinc", Fixtures::KOTLIN).await;
}

#[tokio::test]
async fn csharp_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("cs", "dotnet", Fixtures::CSHARP).await;
}

#[tokio::test]
async fn rust_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("rs", "rustc", Fixtures::RUST).await;
}

#[tokio::test]
async fn go_infers_constructor_dependencies_and_preserves_scope_identity() {
    assert_inferred_graph("go", "go", Fixtures::GO).await;
}

#[tokio::test]
async fn dart_transient_lifetime_gives_each_needs_field_its_own_service() {
    assert_lifetime_effect("dart", "dart", Fixtures::DART, "@Lifetime(\"transient\")\n").await;
}

#[tokio::test]
async fn go_transient_lifetime_gives_each_needs_field_its_own_service() {
    let documented = Fixtures::GO.replace("// @yon::", "// @");
    assert_lifetime_effect("go", "go", &documented, "// @Lifetime(\"transient\")\n").await;
}

#[tokio::test]
async fn fsharp_transient_lifetime_gives_each_needs_field_its_own_service() {
    assert_lifetime_effect(
        "fs",
        "dotnet",
        Fixtures::FSHARP,
        "[<Lifetime(\"transient\")>]\n",
    )
    .await;
}
