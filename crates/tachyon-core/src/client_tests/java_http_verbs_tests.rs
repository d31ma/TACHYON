#![allow(clippy::expect_used)]

use std::path::Path;

struct JavaHttpVerbsProject;

impl JavaHttpVerbsProject {
    const UNIT: &'static str = "client/tests/OrderServiceTest.java";

    const FILES: [(&'static str, &'static str); 6] = [
        (
            "client/models/ClaimConflict.java",
            "record ClaimConflict(String reason, String holder) {}\n",
        ),
        (
            "client/remotes/OrdersClient.java",
            "@HTTPClient({\"path\":\"/api\",\"setup\":\"OrdersSetup\"})\ninterface OrdersClient {\n    @PostRequest(\"/orders\")\n    Object create(@RequestBody Object order, @RequestHeader(\"X-Tenant\") String tenant);\n    @PutRequest(\"/orders/{id}\")\n    Object replace(@PathVariable String id, @RequestBody Object order);\n    @PatchRequest(\"/orders/{id}\")\n    Object change(@PathVariable(\"id\") long id, @RequestParam(\"view\") String view, @RequestBody Object change);\n    @DeleteRequest(\"/orders/{id}\")\n    Object remove(@PathVariable String id, @RequestHeader(\"X-Reason\") String reason);\n    @GetRequest(\"/orders\")\n    Object search(@RequestParam(\"q\") String text, @RequestParam(\"page\") long page);\n    @PutRequest(\"/claims/{id}\")\n    Object claim(@PathVariable String id, @RequestBody Object claim);\n}\n",
        ),
        (
            "client/setups/OrdersSetup.java",
            "@Setup\nclass OrdersSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), (call, current) -> true);\n    }\n}\n",
        ),
        (
            "client/services/OrderService.java",
            "@Service\nclass OrderService {\n    @Needs\n    private final OrdersClient ordersClient;\n    java.util.Map<?, ?> create() { return (java.util.Map<?, ?>)ordersClient.create(java.util.Map.of(\"item\", \"anvil\"), \"north\"); }\n    java.util.Map<?, ?> replace() { return (java.util.Map<?, ?>)ordersClient.replace(\"a-7\", java.util.Map.of(\"item\", \"hammer\")); }\n    java.util.Map<?, ?> change() { return (java.util.Map<?, ?>)ordersClient.change(7L, \"short\", java.util.Map.of(\"count\", \"2\")); }\n    java.util.Map<?, ?> remove() { return (java.util.Map<?, ?>)ordersClient.remove(\"a-7\", \"duplicate\"); }\n    java.util.Map<?, ?> search() { return (java.util.Map<?, ?>)ordersClient.search(\"anvil\", 2L); }\n    Object claim(String id) { return ordersClient.claim(id, java.util.Map.of(\"item\", \"saw\")); }\n}\n",
        ),
        (
            "client/tests/OrderTestSetup.java",
            "@TestSetup\nclass OrderTestSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        Behaviors.HttpAdapter echo = (request, current) -> {\n            if (request.get(\"path\").equals(\"/api/claims/a-7\")) throw new Behaviors.Failure(\"HTTP_STATUS\", null, \"private fake text\", 409, java.util.Map.of(\"reason\", \"taken\", \"holder\", \"south\"));\n            if (request.get(\"path\").equals(\"/api/claims/b-8\")) throw new Behaviors.Failure(\"HTTP_STATUS\", null, \"private fake text\", 200, null);\n            if (request.get(\"path\").equals(\"/api/claims/c-9\")) throw new Behaviors.Failure(\"HTTP_STATUS\", null, \"private fake text\", 404, null);\n            if (request.get(\"path\").equals(\"/api/claims/d-1\")) throw new Behaviors.Failure(\"HTTP_STATUS\", null, \"private fake text\", 409, java.util.List.of(\"taken\"));\n            return java.util.Map.of(\n                \"method\", request.get(\"method\"),\n                \"path\", request.get(\"path\"),\n                \"query\", request.get(\"query\"),\n                \"headers\", request.get(\"headers\"),\n                \"body\", request.getOrDefault(\"body\", \"none\"));\n        };\n        return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(\"orders\", echo), java.util.Map.of(), (call, current) -> true);\n    }\n}\n",
        ),
        (
            "client/tests/OrderServiceTest.java",
            "@Test(OrderTestSetup.class, {\"partition\":\"java-http-verbs\"})\nclass OrderServiceTest {\n    @Fake\n    OrdersClient ordersClient = null;\n    void testPostSendsTheBodyAndAHeader() {\n        java.util.Map<?, ?> echo = construct(OrderService.class).create();\n        Assert.equals(\"POST\", echo.get(\"method\"));\n        Assert.equals(\"/api/orders\", echo.get(\"path\"));\n        Assert.equals(java.util.Map.of(), echo.get(\"query\"));\n        Assert.equals(java.util.Map.of(\"X-Tenant\", \"north\"), echo.get(\"headers\"));\n        Assert.equals(java.util.Map.of(\"item\", \"anvil\"), echo.get(\"body\"));\n    }\n    void testPutFillsThePathAndSendsTheBody() {\n        java.util.Map<?, ?> echo = construct(OrderService.class).replace();\n        Assert.equals(\"PUT\", echo.get(\"method\"));\n        Assert.equals(\"/api/orders/a-7\", echo.get(\"path\"));\n        Assert.equals(java.util.Map.of(\"item\", \"hammer\"), echo.get(\"body\"));\n    }\n    void testPatchFillsANamedPathVariableAndSendsAQueryValue() {\n        java.util.Map<?, ?> echo = construct(OrderService.class).change();\n        Assert.equals(\"PATCH\", echo.get(\"method\"));\n        Assert.equals(\"/api/orders/7\", echo.get(\"path\"));\n        Assert.equals(java.util.Map.of(\"view\", \"short\"), echo.get(\"query\"));\n        Assert.equals(java.util.Map.of(\"count\", \"2\"), echo.get(\"body\"));\n    }\n    void testDeleteFillsThePathAndSendsAHeaderWithoutABody() {\n        java.util.Map<?, ?> echo = construct(OrderService.class).remove();\n        Assert.equals(\"DELETE\", echo.get(\"method\"));\n        Assert.equals(\"/api/orders/a-7\", echo.get(\"path\"));\n        Assert.equals(java.util.Map.of(\"X-Reason\", \"duplicate\"), echo.get(\"headers\"));\n        Assert.equals(\"none\", echo.get(\"body\"));\n    }\n    void testGetSendsEveryRequestParam() {\n        java.util.Map<?, ?> echo = construct(OrderService.class).search();\n        Assert.equals(\"GET\", echo.get(\"method\"));\n        Assert.equals(\"/api/orders\", echo.get(\"path\"));\n        Assert.equals(\"anvil\", ((java.util.Map<?, ?>)echo.get(\"query\")).get(\"q\"));\n        Assert.equals(\"2\", String.valueOf(((java.util.Map<?, ?>)echo.get(\"query\")).get(\"page\")));\n        Assert.equals(\"none\", echo.get(\"body\"));\n    }\n    void testErrorBodyIsReadIntoAModel() {\n        Behaviors.Failure failure = failure(\"a-7\");\n        ClaimConflict conflict = Behaviors.errorBody(failure, TacModels::readClaimConflict).orElseThrow();\n        Assert.equals(\"HTTP_STATUS\", failure.code);\n        Assert.equals(409, Behaviors.errorStatus(failure));\n        Assert.equals(\"The HTTP request failed.\", failure.getMessage());\n        Assert.equals(\"taken\", conflict.reason());\n        Assert.equals(\"south\", conflict.holder());\n    }\n    void testErrorBodyIsEmptyWhenItIsMissingOrDoesNotFit() {\n        Behaviors.Failure missing = failure(\"c-9\");\n        Assert.equals(\"HTTP_STATUS\", missing.code);\n        Assert.equals(404, Behaviors.errorStatus(missing));\n        Assert.equals(false, Behaviors.errorBody(missing, TacModels::readClaimConflict).isPresent());\n        Assert.equals(false, Behaviors.errorBody(failure(\"d-1\"), TacModels::readClaimConflict).isPresent());\n        Assert.equals(false, Behaviors.errorBody(new IllegalStateException(\"other\"), TacModels::readClaimConflict).isPresent());\n        Assert.equals(0, Behaviors.errorStatus(new IllegalStateException(\"other\")));\n    }\n    void testAnErrorStatusOutsideFourHundredToFiveHundredNinetyNineIsAnAdapterFailure() {\n        Behaviors.Failure failure = failure(\"b-8\");\n        Assert.equals(\"http-adapter\", failure.code);\n        Assert.equals(0, Behaviors.errorStatus(failure));\n    }\n    private Behaviors.Failure failure(String id) {\n        try { construct(OrderService.class).claim(id); } catch (Behaviors.Failure error) { return error; }\n        throw new AssertionError(\"expected a failure\");\n    }\n}\n",
        ),
    ];

    fn javac_missing() -> bool {
        std::process::Command::new("javac")
            .arg("-version")
            .output()
            .is_err()
    }

    fn report() -> super::TestReport {
        let root = tempfile::tempdir().expect("test project");
        for (relative, source) in Self::FILES {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("fixture parent"))
                .expect("fixture directory");
            std::fs::write(path, source).expect("fixture source");
        }
        super::run(root.path(), Path::new(Self::UNIT)).expect("native Java test unit")
    }
}

#[test]
fn java_http_clients_send_every_verb_and_request_part_through_the_test_adapter() {
    if JavaHttpVerbsProject::javac_missing() {
        assert!(std::env::var_os("CI").is_none(), "CI must provide javac");
        eprintln!("skipping: javac is unavailable");
        return;
    }
    let report = JavaHttpVerbsProject::report();
    assert_eq!(report.cases.len(), 8, "{report:?}");
    assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
}
