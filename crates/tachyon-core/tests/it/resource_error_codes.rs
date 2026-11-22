#![allow(clippy::expect_used)]

use serde_json::{Value, json};
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    ApplicationConfig, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions,
};

struct CodeCase {
    extension: &'static str,
    program: &'static str,
    source: &'static str,
}

impl CodeCase {
    const SQLITE: &'static str = r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}"#;
    const MESSAGE: &'static str = "Yon managed resource operation failed.";

    async fn run(&self) {
        if !Self::available(self.program) {
            return;
        }
        let project = Self::project(Self::SQLITE);
        let source = Self::worker(&project, self.extension, self.source);
        let sql = Self::answer(&source, project.path(), HttpMethod::Get).await;
        assert_eq!(
            sql,
            json!({
                "conflict": "conflict",
                "too_large": "too_large",
                "transaction_refused": "transaction_refused",
                "budget_spent": "budget_spent",
                "message": Self::MESSAGE,
            }),
            "{}",
            self.extension
        );
        self.check_not_found(&source).await;
    }

    async fn check_not_found(&self, source: &HandlerSource) {
        if !Self::fylo_available() {
            return;
        }
        let directory = tempfile::tempdir().expect("fylo root");
        let root = std::fs::canonicalize(directory.path()).expect("canonical root");
        let path = root.display().to_string().replace('\\', "/");
        let path = path.strip_prefix("//?/").unwrap_or(&path).to_owned();
        let url = format!("fylo:///{}", path.trim_start_matches('/'));
        let configuration = json!({"datasource": {"driver": "fylo", "url": url}}).to_string();
        let fylo_project = Self::project(&configuration);
        let missing = Self::answer(source, fylo_project.path(), HttpMethod::Post).await;
        assert_eq!(
            missing,
            json!({"not_found": "not_found"}),
            "{}",
            self.extension
        );
    }

    fn worker(project: &tempfile::TempDir, extension: &str, source: &str) -> HandlerSource {
        let entry = format!("server/workers/codes.{extension}");
        std::fs::create_dir_all(project.path().join("server/workers")).expect("workers");
        std::fs::write(project.path().join(&entry), source).expect("worker");
        HandlerSource::discover(project.path(), &entry).expect("worker source")
    }

    fn project(application: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        std::fs::create_dir_all(root.path().join("server")).expect("server");
        std::fs::write(root.path().join("server/application.json"), application)
            .expect("configuration");
        root
    }

    async fn answer(source: &HandlerSource, root: &std::path::Path, method: HttpMethod) -> Value {
        let options = HandlerSupervisorOptions {
            application: ApplicationConfig::load(root).expect("configuration"),
            ..HandlerSupervisorOptions::default()
        };
        let supervisor = HandlerSupervisor::new(options).expect("supervisor");
        let request = HandlerRequest::route("resource_codes", "/codes", method);
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .expect("invocation");
        assert_eq!(response.status, 200, "{:?}", response.error);
        serde_json::from_str(&response.body.expect("body").data).expect("JSON answer")
    }

    fn fylo_available() -> bool {
        let available = std::process::Command::new("fylo")
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success());
        assert!(
            available || std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
            "fylo binary is required on this runner"
        );
        available
    }

    fn available(program: &str) -> bool {
        let paths = std::env::var_os("PATH").unwrap_or_default();
        let found = std::env::split_paths(&paths).any(|directory| {
            directory.join(program).is_file()
                || (cfg!(windows)
                    && ["exe", "bat", "cmd"]
                        .iter()
                        .any(|suffix| directory.join(format!("{program}.{suffix}")).is_file()))
        });
        assert!(
            found || std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").as_deref() != Ok("1"),
            "Required resource runtime {program} is unavailable"
        );
        found
    }
}

#[tokio::test]
async fn a_go_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "go",
        program: "go",
        source: r#"
type Handler struct{}

func (Handler) codeOf(err error) string {
	failure, isResource := err.(YonResourceError)
	if !isResource {
		return "none"
	}
	return failure.Code
}

func (handler Handler) GET() any {
	source := DataSource{}
	source.Execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", nil)
	source.Execute("INSERT INTO items (id) VALUES (1)", nil)
	_, conflict := source.Execute("INSERT INTO items (id) VALUES (1)", nil)
	_, tooLarge := source.Query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n", nil)
	_, refused := source.Command(map[string]any{"operation": "transaction", "action": "commit", "transaction": "missing"})
	_, spent := source.Query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n", nil)
	return map[string]any{"conflict": handler.codeOf(conflict), "too_large": handler.codeOf(tooLarge), "transaction_refused": handler.codeOf(refused), "budget_spent": handler.codeOf(spent), "message": conflict.Error()}
}

func (handler Handler) POST() any {
	_, missing := DataSource{}.Insert("missing-things", map[string]any{"name": "a"})
	return map[string]any{"not_found": handler.codeOf(missing)}
}
"#,
    }
    .run()
    .await;
}

#[tokio::test]
async fn a_rust_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "rs",
        program: "rustc",
        source: r#"
struct Handler;
impl Handler {
    fn code_of(result: Result<Json, YonResourceError>) -> &'static str {
        match result {
            Ok(_) => "none",
            Err(error) => error.code(),
        }
    }
    fn GET(request: &__YonFrame) -> Json {
        let source = DataSource::new();
        let _ = source.execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", vec![]);
        let _ = source.execute("INSERT INTO items (id) VALUES (1)", vec![]);
        let conflict = source.execute("INSERT INTO items (id) VALUES (1)", vec![]);
        let message: &'static str = Box::leak(conflict.as_ref().err().map(ToString::to_string).unwrap_or_default().into_boxed_str());
        let too_large = source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n", vec![]);
        let refused = source.command(Json::map([("operation", "transaction"), ("action", "commit"), ("transaction", "missing")]));
        let spent = source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n", vec![]);
        let _ = request;
        Json::map([
            ("conflict", Self::code_of(conflict)),
            ("too_large", Self::code_of(too_large)),
            ("transaction_refused", Self::code_of(refused)),
            ("budget_spent", Self::code_of(spent)),
            ("message", message),
        ])
    }
    fn POST(request: &__YonFrame) -> Json {
        let _ = request;
        let missing = DataSource::new().insert("missing-things", Json::map([("name", "a")]));
        Json::map([("not_found", Self::code_of(missing))])
    }
}
"#,
    }
    .run()
    .await;
}

#[tokio::test]
async fn rust_handler_code_written_before_the_codes_still_compiles() {
    if !CodeCase::available("rustc") {
        return;
    }
    let project = CodeCase::project(CodeCase::SQLITE);
    let source = CodeCase::worker(
        &project,
        "rs",
        r#"
struct Handler;
impl Handler {
    fn old_failure() -> Result<Json, YonResourceError> {
        Err(YonResourceError)
    }
    fn GET(request: &__YonFrame) -> Json {
        let _ = request;
        let error = Self::old_failure().map_err(|_| YonResourceError).err().unwrap_or(YonResourceError);
        let same = if matches!(error, YonResourceError) { "yes" } else { "no" };
        Json::map([("code", error.code()), ("same", same), ("message", "x")])
    }
}
"#,
    );
    let answer = CodeCase::answer(&source, project.path(), HttpMethod::Get).await;
    assert_eq!(
        answer,
        json!({"code": "failed", "same": "yes", "message": "x"})
    );
}

#[tokio::test]
async fn a_java_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "java",
        program: "java",
        source: r#"
class Handler {
    interface Call { Object run(); }
    static String codeOf(Call call) {
        try { call.run(); return "none"; } catch (YonResourceException error) { return error.code(); }
    }
    static Object GET() {
        DataSource source = new DataSource();
        source.execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", java.util.List.of());
        source.execute("INSERT INTO items (id) VALUES (1)", java.util.List.of());
        String message;
        try { source.execute("INSERT INTO items (id) VALUES (1)", java.util.List.of()); message = "none"; } catch (IllegalStateException error) { message = error.getMessage(); }
        return java.util.Map.of(
            "conflict", codeOf(() -> source.execute("INSERT INTO items (id) VALUES (1)", java.util.List.of())),
            "too_large", codeOf(() -> source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n", java.util.List.of())),
            "transaction_refused", codeOf(() -> source.command(java.util.Map.of("operation", "transaction", "action", "commit", "transaction", "missing"))),
            "budget_spent", codeOf(() -> source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n", java.util.List.of())),
            "message", message);
    }
    static Object POST() {
        return java.util.Map.of("not_found", codeOf(() -> new DataSource().insert("missing-things", java.util.Map.of("name", "a"))));
    }
}
"#,
    }
    .run()
    .await;
}

#[tokio::test]
async fn a_kotlin_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "kt",
        program: "kotlinc",
        source: r#"
object Handler {
    private fun codeOf(call: () -> Any?): String =
        try { call(); "none" } catch (error: YonResourceException) { error.code }

    @JvmStatic fun GET(): Any? {
        val source = DataSource()
        source.execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", emptyList())
        source.execute("INSERT INTO items (id) VALUES (1)", emptyList())
        val message = try { source.execute("INSERT INTO items (id) VALUES (1)", emptyList()); "none" } catch (error: IllegalStateException) { error.message }
        return mapOf(
            "conflict" to codeOf { source.execute("INSERT INTO items (id) VALUES (1)", emptyList()) },
            "too_large" to codeOf { source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n", emptyList()) },
            "transaction_refused" to codeOf { source.command(mapOf("operation" to "transaction", "action" to "commit", "transaction" to "missing")) },
            "budget_spent" to codeOf { source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n", emptyList()) },
            "message" to message,
        )
    }

    @JvmStatic fun POST(): Any? = mapOf("not_found" to codeOf { DataSource().insert("missing-things", mapOf("name" to "a")) })
}
"#,
    }
    .run()
    .await;
}

#[tokio::test]
async fn a_csharp_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "cs",
        program: "dotnet",
        source: r#"
sealed class Handler
{
    static string CodeOf(System.Func<object> call)
    {
        try { call(); return "none"; } catch (YonResourceException error) { return error.Code; }
    }

    public static object GET()
    {
        var source = new DataSource();
        source.Execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", new object[] { });
        source.Execute("INSERT INTO items (id) VALUES (1)", new object[] { });
        string message;
        try { source.Execute("INSERT INTO items (id) VALUES (1)", new object[] { }); message = "none"; } catch (System.InvalidOperationException error) { message = error.Message; }
        return new System.Collections.Generic.Dictionary<string, string>
        {
            ["conflict"] = CodeOf(() => source.Execute("INSERT INTO items (id) VALUES (1)", new object[] { })),
            ["too_large"] = CodeOf(() => source.Query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n", new object[] { })),
            ["transaction_refused"] = CodeOf(() => source.Command(new { operation = "transaction", action = "commit", transaction = "missing" })),
            ["budget_spent"] = CodeOf(() => source.Query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n", new object[] { })),
            ["message"] = message,
        };
    }

    public static object POST() => new System.Collections.Generic.Dictionary<string, string>
    {
        ["not_found"] = CodeOf(() => new DataSource().Insert("missing-things", new { name = "a" })),
    };
}
"#,
    }
    .run()
    .await;
}

#[tokio::test]
async fn a_php_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "php",
        program: "php-cgi",
        source: r"<?php
final class Handler
{
    private static function codeOf(callable $call): string
    {
        try { $call(); return 'none'; } catch (YonResourceException $error) { return $error->resourceCode(); }
    }

    public static function GET(): array
    {
        $source = new DataSource();
        $source->execute('CREATE TABLE items (id INTEGER PRIMARY KEY)', []);
        $source->execute('INSERT INTO items (id) VALUES (1)', []);
        try { $source->execute('INSERT INTO items (id) VALUES (1)', []); $message = 'none'; } catch (RuntimeException $error) { $message = $error->getMessage(); }
        return [
            'conflict' => self::codeOf(fn () => $source->execute('INSERT INTO items (id) VALUES (1)', [])),
            'too_large' => self::codeOf(fn () => $source->query('WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n', [])),
            'transaction_refused' => self::codeOf(fn () => $source->command(['operation' => 'transaction', 'action' => 'commit', 'transaction' => 'missing'])),
            'budget_spent' => self::codeOf(fn () => $source->query('WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n', [])),
            'message' => $message,
        ];
    }

    public static function POST(): array
    {
        return ['not_found' => self::codeOf(fn () => (new DataSource())->insert('missing-things', ['name' => 'a']))];
    }
}
",
    }
    .run()
    .await;
}

#[tokio::test]
async fn a_dart_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "dart",
        program: "dart",
        source: r"
class Handler {
  Future<String> codeOf(Future<Object?> Function() call) async {
    try {
      await call();
      return 'none';
    } on YonResourceError catch (error) {
      return error.code;
    }
  }

  Future<Object?> GET() async {
    final source = DataSource();
    await source.execute('CREATE TABLE items (id INTEGER PRIMARY KEY)', []);
    await source.execute('INSERT INTO items (id) VALUES (1)', []);
    var message = 'none';
    try {
      await source.execute('INSERT INTO items (id) VALUES (1)', []);
    } on StateError catch (error) {
      message = error.message;
    }
    return {
      'conflict': await codeOf(() => source.execute('INSERT INTO items (id) VALUES (1)', [])),
      'too_large': await codeOf(() => source.query('WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n', [])),
      'transaction_refused': await codeOf(() => source.command({'operation': 'transaction', 'action': 'commit', 'transaction': 'missing'})),
      'budget_spent': await codeOf(() => source.query('WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n', [])),
      'message': message,
    };
  }

  Future<Object?> POST() async =>
      {'not_found': await codeOf(() => DataSource().insert('missing-things', {'name': 'a'}))};
}
",
    }
    .run()
    .await;
}

#[tokio::test]
async fn a_javascript_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "js",
        program: "node",
        source: r"
export class Handler {
  static async codeOf(call) {
    try {
      await call()
      return 'none'
    } catch (error) {
      return error.code
    }
  }

  static async GET(request) {
    const source = new DataSource()
    await source.execute('CREATE TABLE items (id INTEGER PRIMARY KEY)', [])
    await source.execute('INSERT INTO items (id) VALUES (1)', [])
    let message = 'none'
    try { await source.execute('INSERT INTO items (id) VALUES (1)', []) } catch (error) { message = error.message }
    return {
      conflict: await Handler.codeOf(() => source.execute('INSERT INTO items (id) VALUES (1)', [])),
      too_large: await Handler.codeOf(() => source.query('WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n', [])),
      transaction_refused: await Handler.codeOf(() => source.command({ operation: 'transaction', action: 'commit', transaction: 'missing' })),
      budget_spent: await Handler.codeOf(() => source.query('WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n', [])),
      message,
    }
  }

  static async POST(request) {
    return { not_found: await Handler.codeOf(() => new DataSource().insert('missing-things', { name: 'a' })) }
  }
}
",
    }
    .run()
    .await;
}

#[tokio::test]
async fn a_python_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "py",
        program: "python3",
        source: r#"
class Handler:
    @staticmethod
    def code_of(call):
        try:
            call()
        except Exception as error:
            return error.code
        return "none"

    @staticmethod
    def GET(request):
        source = DataSource()
        source.execute("CREATE TABLE items (id INTEGER PRIMARY KEY)")
        source.execute("INSERT INTO items (id) VALUES (1)")
        try:
            source.execute("INSERT INTO items (id) VALUES (1)")
            message = "none"
        except RuntimeError as error:
            message = str(error)
        return {
            "conflict": Handler.code_of(lambda: source.execute("INSERT INTO items (id) VALUES (1)")),
            "too_large": Handler.code_of(lambda: source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n")),
            "transaction_refused": Handler.code_of(lambda: source.command({"operation": "transaction", "action": "commit", "transaction": "missing"})),
            "budget_spent": Handler.code_of(lambda: source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n")),
            "message": message,
        }

    @staticmethod
    def POST(request):
        return {"not_found": Handler.code_of(lambda: DataSource().insert("missing-things", {"name": "a"}))}
"#,
    }
    .run()
    .await;
}

#[tokio::test]
async fn a_ruby_handler_reads_each_resource_error_code() {
    CodeCase {
        extension: "rb",
        program: "ruby",
        source: r#"
class Handler
  def self.code_of
    yield
    "none"
  rescue YonResourceError => error
    error.code
  end

  def self.GET(request)
    source = DataSource.new
    source.execute("CREATE TABLE items (id INTEGER PRIMARY KEY)")
    source.execute("INSERT INTO items (id) VALUES (1)")
    message = begin
      source.execute("INSERT INTO items (id) VALUES (1)")
      "none"
    rescue StandardError => error
      error.message
    end
    {
      "conflict" => code_of { source.execute("INSERT INTO items (id) VALUES (1)") },
      "too_large" => code_of { source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n") },
      "transaction_refused" => code_of { source.command({ "operation" => "transaction", "action" => "commit", "transaction" => "missing" }) },
      "budget_spent" => code_of { source.query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n") },
      "message" => message,
    }
  end

  def self.POST(request)
    { "not_found" => code_of { DataSource.new.insert("missing-things", { "name" => "a" }) } }
  end
end
"#,
    }
    .run()
    .await;
}
