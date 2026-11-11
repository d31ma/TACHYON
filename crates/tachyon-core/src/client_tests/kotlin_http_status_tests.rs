#![allow(clippy::expect_used)]

use std::path::Path;

struct KotlinHttpStatusProject;

impl KotlinHttpStatusProject {
    const UNIT: &'static str = "client/tests/ProblemServiceTest.kt";
    const CASES: usize = 6;
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/models/Greeting.kt",
            "data class Greeting(val message: String)\n",
        ),
        (
            "client/remotes/ProblemClient.kt",
            "import models.Greeting\n\n@HTTPClient({\"path\":\"/api\",\"adapter\":\"problems\"})\ninterface ProblemClient {\n    @GetRequest(\"/problems/{kind}\")\n    suspend fun read(@PathVariable(\"kind\") kind: String): Greeting\n}\n",
        ),
        (
            "client/tests/ProblemTestSetup.kt",
            "@TestSetup\nclass ProblemTestSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =\n            Behaviors.Configuration(http = mapOf(\"problems\" to Behaviors.HttpAdapter { request, _ ->\n                throw mapOf(\n                    \"/api/problems/success\" to Behaviors.Failure(\"HTTP_STATUS\", null, \"private\", 200, mapOf(\"message\" to \"taken\")),\n                    \"/api/problems/unsafe\" to Behaviors.Failure(\"HTTP_STATUS\", null, \"private\", 422, Double.NaN),\n                    \"/api/problems/empty\" to Behaviors.Failure(\"HTTP_STATUS\", null, \"private\", 404),\n                    \"/api/problems/list\" to Behaviors.Failure(\"HTTP_STATUS\", null, \"private\", 409, listOf(\"taken\")),\n                    \"/api/problems/storage\" to Behaviors.Failure(\"storage\", null, \"kept\"),\n                ).getOrElse(request[\"path\"] as String) { IllegalStateException(\"private\") }\n            }))\n    }\n}\n",
        ),
        (
            "client/tests/ProblemServiceTest.kt",
            "import models.Greeting\n\n@Test(ProblemTestSetup::class, {\"partition\":\"tests\"})\nclass ProblemServiceTest {\n    @Fake\n    var problemClient: ProblemClient? = null\n\n    suspend fun testASuccessStatusIsAnAdapterFailure() {\n        val error = failure(\"success\")\n        Assert.equals(\"http-adapter\", error.code)\n        Assert.equals(0, error.status)\n    }\n\n    suspend fun testAnUnsafeBodyIsAnAdapterFailure() {\n        Assert.equals(\"http-adapter\", failure(\"unsafe\").code)\n    }\n\n    suspend fun testAPlainErrorIsAnAdapterFailure() {\n        val error = failure(\"plain\")\n        Assert.equals(\"http-adapter\", error.code)\n        Assert.isTrue(!error.message.orEmpty().contains(\"private\"))\n    }\n\n    suspend fun testAStatusWithoutABodyHasNoErrorBody() {\n        val error = failure(\"empty\")\n        Assert.equals(\"HTTP_STATUS\", error.code)\n        Assert.equals(404, error.status)\n        Assert.equals(\"The HTTP request failed.\", error.message)\n        Assert.equals(null, Behaviors.errorBody<Greeting>(error))\n    }\n\n    suspend fun testABodyOfTheWrongShapeIsNotRead() {\n        val error = failure(\"list\")\n        Assert.equals(409, error.status)\n        Assert.equals(null, Behaviors.errorBody<Greeting>(error))\n        Assert.equals(null, Behaviors.errorBody<Greeting>(IllegalStateException(\"plain\")))\n        Assert.equals(null, Behaviors.errorBody<Greeting>(null))\n    }\n\n    suspend fun testAnotherFailureCodePassesAsBefore() {\n        val error = failure(\"storage\")\n        Assert.equals(\"storage\", error.code)\n        Assert.equals(\"kept\", error.message)\n    }\n\n    private suspend fun failure(kind: String): Behaviors.Failure =\n        runCatching { construct(ProblemService::class).read(kind) }.exceptionOrNull() as Behaviors.Failure\n}\n",
        ),
    ];
    const SERVICE: (&'static str, &'static str) = (
        "client/services/ProblemService.kt",
        "import models.Greeting\n\n@Service\nclass ProblemService {\n    @Needs\n    private val problemClient: ProblemClient\n\n    suspend fun read(kind: String): Greeting = problemClient.read(kind)\n}\n",
    );

    fn write(root: &Path) {
        for (relative, source) in Self::FILES.into_iter().chain([Self::SERVICE]) {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
    }
}

#[test]
fn kotlin_http_adapters_pass_an_error_status_only_when_it_is_a_valid_one() {
    if crate::kotlin_home::KotlinHome::command("kotlinc")
        .arg("-version")
        .output()
        .is_err()
    {
        assert!(std::env::var_os("CI").is_none(), "CI must provide kotlinc");
        eprintln!("skipping: kotlinc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("test project");
    KotlinHttpStatusProject::write(root.path());
    let report = super::run(root.path(), Path::new(KotlinHttpStatusProject::UNIT))
        .expect("native Kotlin test unit");
    assert_eq!(
        report.cases.len(),
        KotlinHttpStatusProject::CASES,
        "{report:?}"
    );
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
