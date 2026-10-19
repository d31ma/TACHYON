#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerRequest, HttpMethod};
use tachyon_core::{
    ApplicationConfig, HandlerCancellation, HandlerSource, HandlerSupervisor,
    HandlerSupervisorOptions,
};

struct Targets;

impl Targets {
    const PYTHON: (&str, &str) = (
        "server/services/ReportService.py",
        r#"import json
import os


@service
class ReportService:
    LANGUAGE = {"language": "python"}

    report_archive_service: Annotated[ReportArchiveService, needs]

    def summarize(self, mode, detail):
        self._fail_when(mode)
        self._hang_when(mode, detail)
        return self._answer(mode, detail)

    def _fail_when(self, mode):
        if mode == "fail":
            raise RuntimeError("secret-canary target failure")

    def _hang_when(self, mode, detail):
        if mode == "hang":
            self._hang(detail)

    def _hang(self, detail):
        program = "sleep"
        child = os.spawnlp(os.P_NOWAIT, program, program, "30")
        with open(detail, "w") as target:
            target.write(str(child))
        os.waitpid(child, 0)

    def _answer(self, mode, detail):
        answers = {"hop": self._hop, "headers": self._headers}
        return answers.get(mode, self._stored)(mode, detail)

    def _hop(self, mode, detail):
        return YonGateway.call("ReportClient.summarize", ["plain", None])

    def _headers(self, mode, detail):
        return {**self.LANGUAGE, "seen": detail}

    def _stored(self, mode, detail):
        stored = self.report_archive_service.store(detail)
        return {**self.LANGUAGE, "mode": mode, "detail": detail, "stored": stored}

    def count(self):
        return 3
"#,
    );

    const PYTHON_DATA_SERVICE: (&str, &str) = (
        "server/services/ReportArchiveService.py",
        r#"@service
class ReportArchiveService:
    data_source: Annotated[DataSource, needs]

    def store(self, detail):
        self.data_source.execute("CREATE TABLE IF NOT EXISTS reports (detail TEXT)", [])
        self.data_source.execute("INSERT INTO reports (detail) VALUES (?)", [str(detail)])
        rows = self.data_source.query("SELECT COUNT(*) AS total FROM reports", [])
        return rows[0]["total"]
"#,
    );

    const RUBY: (&str, &str) = (
        "server/services/ReportService.rb",
        r#"#[Service]
class ReportService
  def summarize(mode, detail)
    fail_when(mode)
    { "language" => "ruby", "mode" => mode, "detail" => detail }
  end

  private

  def fail_when(mode)
    raise "secret-canary target failure" if mode == "fail"
  end
end
"#,
    );

    const JAVASCRIPT: (&str, &str) = (
        "server/services/ReportService.js",
        r"@Service
class ReportService {
  async summarize(mode, detail) {
    this.#failWhen(mode)
    return { language: 'javascript', mode, detail }
  }

  #failWhen(mode) {
    if (mode === 'fail') throw new Error('secret-canary target failure')
  }
}
",
    );

    const PHP: (&str, &str) = (
        "server/services/ReportService.php",
        r"<?php
#[Service]
final class ReportService
{
    public function summarize($mode, $detail)
    {
        $this->failWhen($mode);
        return ['language' => 'php', 'mode' => $mode, 'detail' => $detail];
    }

    private function failWhen($mode): void
    {
        if ($mode === 'fail') {
            throw new RuntimeException('secret-canary target failure');
        }
    }
}
",
    );

    const JAVA: (&str, &str) = (
        "server/services/ReportService.java",
        r#"@Service
@NeedsAll()
class ReportService {
    ReportArchiveService reportArchiveService;
    Object summarize(Object mode, Object detail) {
        failWhen(mode);
        return answerFor(mode, detail);
    }
    private void failWhen(Object mode) {
        if ("fail".equals(mode)) throw new IllegalStateException("secret-canary target failure");
    }
    private Object answerFor(Object mode, Object detail) {
        if ("nothing".equals(mode)) return null;
        return answer(mode, detail);
    }
    private Object answer(Object mode, Object detail) {
        java.util.Map<String, Object> answer = new java.util.LinkedHashMap<>();
        answer.put("language", "java");
        answer.put("mode", mode);
        answer.put("detail", detail);
        answer.put("stored", reportArchiveService.store(detail));
        answer.put("shape", shape(java.util.Map.of("id", 7L)));
        return answer;
    }
    @Convert({"fields":[{"target":"kind","constant":"report"}]})
    Object shape(Object value) {}
}
"#,
    );

    const JAVA_DATA_SERVICE: (&str, &str) = (
        "server/services/ReportArchiveService.java",
        r#"@Service
@NeedsAll()
class ReportArchiveService {
    DataSource dataSource;
    Object store(Object detail) {
        dataSource.execute("CREATE TABLE IF NOT EXISTS reports (detail TEXT)", java.util.List.of());
        dataSource.execute("INSERT INTO reports (detail) VALUES (?)", java.util.List.of(String.valueOf(detail)));
        return dataSource.query("SELECT COUNT(*) AS total FROM reports", java.util.List.of());
    }
}
"#,
    );

    const KOTLIN: (&str, &str) = (
        "server/services/ReportService.kt",
        r#"@Service
class ReportService {
    fun summarize(mode: Any?, detail: Any?): Any? {
        failWhen(mode)
        return answerFor(mode, detail)
    }

    private fun failWhen(mode: Any?) {
        if (mode == "fail") throw IllegalStateException("secret-canary target failure")
    }

    private fun answerFor(mode: Any?, detail: Any?): Any? {
        if (mode == "nothing") return null
        return mapOf("language" to "kotlin", "mode" to mode, "detail" to detail)
    }
}
"#,
    );

    const CSHARP: (&str, &str) = (
        "server/services/ReportService.cs",
        r#"[Service]
class ReportService {
    public BehaviorJSON summarize(BehaviorJSON mode, BehaviorJSON detail) {
        FailWhen(mode);
        return AnswerFor(mode, detail);
    }

    private static void FailWhen(BehaviorJSON mode) {
        if (mode.Text == "fail") throw new System.InvalidOperationException("secret-canary target failure");
    }

    private static BehaviorJSON AnswerFor(BehaviorJSON mode, BehaviorJSON detail) {
        if (mode.Text == "nothing") return null;
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["language"] = BehaviorJSON.From("csharp"), ["mode"] = mode, ["detail"] = detail,
        });
    }
}
"#,
    );

    const RUST: (&str, &str) = (
        "server/services/ReportService.rs",
        r#"#[yon::service]
struct ReportService;
impl ReportService {
    fn new() -> Self { Self }
    fn summarize(&self, mode: BehaviorValue, detail: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        Self::fail_when(&mode)?;
        Ok(Self::answer_for(mode, detail))
    }

    fn fail_when(mode: &BehaviorValue) -> BehaviorResult<()> {
        if *mode == BehaviorValue::from("fail") {
            return Err(BehaviorError::new("failed", "secret-canary target failure"));
        }
        Ok(())
    }

    fn answer_for(mode: BehaviorValue, detail: BehaviorValue) -> BehaviorValue {
        if mode == BehaviorValue::from("nothing") {
            return BehaviorValue::Null;
        }
        BehaviorValue::object([("language", BehaviorValue::from("rust")), ("mode", mode), ("detail", detail)])
    }
}
"#,
    );

    const DART: (&str, &str) = (
        "server/services/ReportService.dart",
        r"@Service
class ReportService {
  Future<Object?> summarize(Object? mode, Object? detail) async {
    _failWhen(mode);
    return _answerFor(mode, detail);
  }

  void _failWhen(Object? mode) {
    if (mode == 'fail') throw StateError('secret-canary target failure');
  }

  Object? _answerFor(Object? mode, Object? detail) {
    if (mode == 'nothing') return null;
    return {'language': 'dart', 'mode': mode, 'detail': detail};
  }
}
",
    );

    const GO: (&str, &str) = (
        "server/services/ReportService.go",
        r#"import (
	"errors"
	"os"
	"os/exec"
	"strconv"
)

// @Service
type ReportService struct {
	// @Needs
	reportArchiveService *ReportArchiveService
}

func (service *ReportService) Summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {
	answer := service.answerFor(mode)
	return answer(mode, detail)
}

func (service *ReportService) answerFor(mode BehaviorValue) func(BehaviorValue, BehaviorValue) (BehaviorValue, error) {
	answers := map[string]func(BehaviorValue, BehaviorValue) (BehaviorValue, error){
		"fail":    service.fail,
		"nothing": service.nothing,
		"hang":    service.hang,
		"hop":     service.hop,
	}
	name, _ := mode.(string)
	answer, known := answers[name]
	if !known {
		return service.stored
	}
	return answer
}

func (service *ReportService) fail(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {
	return nil, &BehaviorError{Code: "failed", Message: "secret-canary target failure"}
}

func (service *ReportService) nothing(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {
	return nil, nil
}

func (service *ReportService) hang(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {
	child := exec.Command("sleep", "30")
	return nil, service.waitFor(child, detail, child.Start())
}

func (service *ReportService) waitFor(child *exec.Cmd, detail BehaviorValue, startFailure error) error {
	if startFailure != nil {
		return startFailure
	}
	return service.recordAndWait(child, detail)
}

func (service *ReportService) recordAndWait(child *exec.Cmd, detail BehaviorValue) error {
	path, _ := detail.(string)
	writeErr := os.WriteFile(path, []byte(strconv.Itoa(child.Process.Pid)), 0o600)
	return errors.Join(writeErr, child.Wait())
}

func (service *ReportService) hop(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {
	return YonGateway.Call("ReportClient.summarize", `["plain",null]`), nil
}

func (service *ReportService) stored(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {
	stored, err := service.reportArchiveService.Store(detail)
	return map[string]any{"language": "go", "mode": mode, "detail": detail, "stored": stored}, err
}
"#,
    );

    const GO_DATA_SERVICE: (&str, &str) = (
        "server/services/ReportArchiveService.go",
        r#"import "fmt"

// @Service
type ReportArchiveService struct {
	dataSource DataSource
}

func (service *ReportArchiveService) Store(detail BehaviorValue) (any, error) {
	if _, err := service.dataSource.Execute("CREATE TABLE IF NOT EXISTS reports (detail TEXT)", nil); err != nil {
		return nil, err
	}
	if _, err := service.dataSource.Execute("INSERT INTO reports (detail) VALUES (?)", []any{fmt.Sprint(detail)}); err != nil {
		return nil, err
	}
	return service.dataSource.Query("SELECT COUNT(*) AS total FROM reports", nil)
}
"#,
    );

    const CPP: (&str, &str) = (
        "server/services/ReportService.hpp",
        r#"#pragma once
#include <stdexcept>
#include <string>

class [[Service]] ReportService {
    [[Needs]] ReportLabels& labels;

public:
    tachyon::json::JsonNode summarize(tachyon::json::JsonNode mode, const tachyon::json::JsonNode& detail) {
        std::string written = mode.getValueKind() == tachyon::json::JsonValueKind::String ? mode.getValue<std::string>() : std::string();
        if (written == "fail") {
            throw std::runtime_error("secret-canary target failure");
        }
        if (written == "nothing") {
            return tachyon::json::JsonNode(nullptr);
        }
        tachyon::json::JsonObject answer;
        answer["language"] = labels.language();
        answer["mode"] = mode;
        answer["detail"] = detail;
        return answer;
    }

    tachyon::json::JsonNode count() const {
        return tachyon::json::JsonNode(3);
    }
};
"#,
    );

    const CPP_HELPER: (&str, &str) = (
        "server/helpers/ReportLabels.hpp",
        r#"#pragma once
#include <string>

class [[Helper]] ReportLabels {
public:
    std::string language() const {
        return "cpp";
    }
};
"#,
    );
}

struct Callers;

impl Callers {
    fn javascript(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.js"),
                format!(
                    "@HTTPClient\nclass ReportClient {{\n  @Gateway({{\"class\":\"{target}\",\"method\":\"summarize\"}})\n  async summarize(mode, detail) {{}}\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.js"),
                String::from(
                    "@RequestRoute(\"/report\")\n@Public\nclass ReportController {\n  @Needs\n  reportClient;\n\n  @PostRoute\n  async postReport(request) {\n    const [mode, detail] = JSON.parse(await request.text())\n    return this.#summarized(mode, detail)\n  }\n\n  async #summarized(mode, detail) {\n    try {\n      return await this.reportClient.summarize(mode, detail)\n    } catch (error) {\n      return { code: error.code, message: error.message }\n    }\n  }\n}\n",
                ),
            ),
        ]
    }

    fn typescript(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.ts"),
                format!(
                    "@HTTPClient\nclass ReportClient {{\n  @Gateway({{\"class\":\"{target}\"}})\n  async summarize(mode: BehaviorJsonValue, detail: BehaviorJsonValue): Promise<BehaviorJsonValue> {{}}\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.ts"),
                String::from(
                    "@RequestRoute(\"/report\")\n@Public\nclass ReportController {\n  @Needs\n  readonly reportClient!: ReportClient;\n\n  @PostRoute\n  async postReport(request: Request) {\n    const [mode, detail] = JSON.parse(await request.text())\n    return this.summarized(mode, detail)\n  }\n\n  private async summarized(mode: BehaviorJsonValue, detail: BehaviorJsonValue) {\n    try {\n      return await this.reportClient.summarize(mode, detail)\n    } catch (error) {\n      return { code: (error as { code: string }).code }\n    }\n  }\n}\n",
                ),
            ),
        ]
    }

    fn php(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.php"),
                format!(
                    "<?php\n#[HTTPClient]\ninterface ReportClient {{\n    #[Gateway({{\"class\":\"{target}\"}})]\n    public function summarize($mode, $detail);\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.php"),
                String::from(
                    "<?php\n#[RequestRoute(\"/report\")]\n#[Public]\nclass ReportController {\n    #[Needs]\n    public ReportClient $reportClient;\n\n    #[PostRoute]\n    public function postReport() {\n        [$mode, $detail] = json_decode(file_get_contents('php://input'), true);\n        return $this->summarized($mode, $detail);\n    }\n\n    private function summarized($mode, $detail) {\n        try {\n            return $this->reportClient->summarize($mode, $detail);\n        } catch (BehaviorError $error) {\n            return ['code' => $error->behaviorCode, 'message' => $error->getMessage()];\n        }\n    }\n}\n",
                ),
            ),
        ]
    }

    fn ruby(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.rb"),
                format!(
                    "#[HTTPClient]\nclass ReportClient\n  #[Gateway({{\"class\":\"{target}\"}})]\n  def summarize(mode, detail)\n  end\nend\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.rb"),
                String::from(
                    "#[RequestRoute(\"/report\")]\n#[Public]\nclass ReportController\n  #[Needs]\n  attr_reader :report_client\n\n  #[PostRoute]\n  def post_report(request)\n    mode, detail = JSON.parse(request.body)\n    summarized(mode, detail)\n  end\n\n  private\n\n  def summarized(mode, detail)\n    report_client.summarize(mode, detail)\n  rescue BehaviorError => error\n    { \"code\" => error.behavior_code, \"message\" => error.message }\n  end\nend\n",
                ),
            ),
        ]
    }

    fn java(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.java"),
                format!(
                    "@HTTPClient\ninterface ReportClient {{\n    @Gateway({{\"class\":\"{target}\"}})\n    Object summarize(Object mode, Object detail);\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.java"),
                String::from(
                    "@RequestRoute(\"/report\")\n@Public\n@NeedsAll()\nclass ReportController {\n    ReportClient reportClient;\n    @PostRoute\n    Object postReport(com.sun.net.httpserver.HttpExchange exchange) throws java.io.IOException {\n        String text = new String(exchange.getRequestBody().readAllBytes(), java.nio.charset.StandardCharsets.UTF_8);\n        java.util.List<?> input = (java.util.List<?>) Json.parse(text);\n        return summarized(input.get(0), input.get(1));\n    }\n    private Object summarized(Object mode, Object detail) {\n        try {\n            return reportClient.summarize(mode, detail);\n        } catch (Behaviors.Failure error) {\n            return java.util.Map.of(\"code\", error.code, \"message\", error.getMessage());\n        }\n    }\n}\n",
                ),
            ),
        ]
    }

    fn kotlin(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.kt"),
                format!(
                    "@HTTPClient\ninterface ReportClient {{\n    @Gateway({{\"class\":\"{target}\"}})\n    fun summarize(mode: Any?, detail: Any?): Any?\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.kt"),
                String::from(
                    "@RequestRoute(\"/report\")\n@Public\n@NeedsAll()\nclass ReportController {\n    val reportClient: ReportClient\n    @PostRoute\n    fun postReport(exchange: com.sun.net.httpserver.HttpExchange): Any? {\n        val input = Json.parse(String(exchange.requestBody.readAllBytes())) as List<*>\n        return summarized(input[0], input[1])\n    }\n    private fun summarized(mode: Any?, detail: Any?): Any? {\n        return try {\n            reportClient.summarize(mode, detail)\n        } catch (error: Behaviors.Failure) {\n            mapOf(\"code\" to error.code, \"message\" to error.message)\n        }\n    }\n}\n",
                ),
            ),
        ]
    }

    fn csharp(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.cs"),
                format!(
                    "[HTTPClient]\npublic interface ReportClient {{\n    [Gateway({{\"class\":\"{target}\"}})]\n    BehaviorJSON summarize(BehaviorJSON mode, BehaviorJSON detail);\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.cs"),
                String::from(
                    "[RequestRoute(\"/report\")]\n[Public]\n[Needs(typeof(ReportClient))]\nclass ReportController {\n    readonly ReportClient reportClient;\n    public ReportController(ReportClient reportClient) { this.reportClient = reportClient; }\n    [PostRoute]\n    public object PostReport(HttpRequestMessage request) {\n        var input = BehaviorJSON.Parse(request.Content.ReadAsStringAsync().Result).Items;\n        return Summarized(input[0], input[1]);\n    }\n    private object Summarized(BehaviorJSON mode, BehaviorJSON detail) {\n        try {\n            return reportClient.summarize(mode, detail);\n        } catch (BehaviorError error) {\n            return new { code = error.Code, message = error.Message };\n        }\n    }\n}\n",
                ),
            ),
        ]
    }

    fn rust(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.rs"),
                format!(
                    "#[HTTPClient]\ntrait ReportClient {{\n    #[Gateway({{\"class\":\"{target}\"}})]\n    fn summarize(&self, mode: BehaviorValue, detail: BehaviorValue) -> BehaviorResult<BehaviorValue>;\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.rs"),
                String::from(
                    "#[RequestRoute(\"/report\")]\n#[Public]\n#[yon::needs(ReportClient)]\nstruct ReportController { report_client: std::sync::Arc<ReportClient> }\nimpl ReportController {\n    fn new(report_client: std::sync::Arc<ReportClient>) -> Self { Self { report_client } }\n    #[PostRoute]\n    fn post_report(&self, request: &http::Request<Vec<u8>>) -> Json {\n        let text = String::from_utf8_lossy(request.body()).into_owned();\n        let input: Vec<BehaviorValue> = Json::parse(&text).expect(\"JSON body\").items().iter().cloned().map(BehaviorValue::from).collect();\n        match self.report_client.summarize(input[0].clone(), input[1].clone()) {\n            Ok(value) => Json::parse(&value.to_json(1_048_576).expect(\"JSON answer\")).expect(\"JSON value\"),\n            Err(error) => Json::map([(\"code\", error.code), (\"message\", error.message)]),\n        }\n    }\n}\n",
                ),
            ),
        ]
    }

    fn dart(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.dart"),
                format!(
                    "@HTTPClient\nabstract interface class ReportClient {{\n  @Gateway({{\"class\":\"{target}\"}})\n  Future<Object?> summarize(Object? mode, Object? detail);\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.dart"),
                String::from(
                    "import 'dart:convert';\nimport 'package:shelf/shelf.dart';\n\n@RequestRoute(\"/report\")\n@Public\nclass ReportController {\n  @Needs\n  final ReportClient reportClient;\n\n  @PostRoute\n  Future<Object?> postReport(Request request) async {\n    final input = jsonDecode(await request.readAsString()) as List;\n    return _summarized(input[0], input[1]);\n  }\n\n  Future<Object?> _summarized(Object? mode, Object? detail) async {\n    try {\n      return await reportClient.summarize(mode, detail);\n    } on BehaviorError catch (error) {\n      return {'code': error.code, 'message': error.message};\n    }\n  }\n}\n",
                ),
            ),
        ]
    }

    fn go(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.go"),
                format!(
                    "// @HTTPClient\ntype ReportClient interface {{\n\t// @Gateway({{\"class\":\"{target}\",\"method\":\"summarize\"}})\n\tSummarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error)\n}}\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.go"),
                String::from(
                    "import (\n\t\"encoding/json\"\n\t\"io\"\n\t\"net/http\"\n)\n\n// @RequestRoute(\"/report\")\n// @Public\ntype ReportController struct {\n\t// @Needs\n\treportClient *ReportClient\n}\n\n// @PostRoute\nfunc (controller *ReportController) PostReport(request *http.Request) any {\n\tinput := []any{nil, nil}\n\tbody, _ := io.ReadAll(request.Body)\n\tjson.Unmarshal(body, &input)\n\tvalue, err := controller.reportClient.Summarize(input[0], input[1])\n\treturn controller.answerOf(value, err)\n}\n\nfunc (controller *ReportController) answerOf(value BehaviorValue, failure error) any {\n\tif failure != nil {\n\t\treturn map[string]any{\"code\": BehaviorErrorCode(failure), \"message\": failure.(*BehaviorError).Message}\n\t}\n\treturn value\n}\n",
                ),
            ),
        ]
    }

    fn with_client(extension: &str, client: &str) -> [(String, String); 2] {
        let target = Targets::PYTHON.0;
        let mut caller = match extension {
            "java" => Self::java(target),
            "kt" => Self::kotlin(target),
            "cs" => Self::csharp(target),
            "rs" => Self::rust(target),
            "dart" => Self::dart(target),
            _ => Self::go(target),
        };
        client.clone_into(&mut caller[0].1);
        caller
    }

    fn lua(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.lua"),
                format!(
                    "-- @HTTPClient\n---@class ReportClient\nReportClient = {{}}\n\n---@param mode any\n---@param detail any\n---@return any\n-- @Gateway({{\"class\":\"{target}\"}})\nfunction ReportClient:summarize(mode, detail)\nend\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.lua"),
                String::from(
                    "-- @RequestRoute(\"/report\")\n-- @Public\n---@class ReportController\nReportController = {\n  -- @Needs\n  ---@type ReportClient\n  report_client = nil,\n}\n\n---@param input any[]\n---@return table\n-- @PostRoute\nfunction ReportController:post_report(\n  -- @FromBody\n  input\n)\n  local ok, answer = pcall(self.report_client.summarize, self.report_client, input[1], input[2])\n  if ok then\n    return answer\n  end\n  return { code = answer.behavior_code, message = answer.message }\nend\n",
                ),
            ),
        ]
    }

    fn cpp(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.hpp"),
                format!(
                    "#pragma once\n\nclass [[HTTPClient]] ReportClient {{\npublic:\n    [[Gateway({{\"class\":\"{target}\"}})]]\n    tachyon::json::JsonNode summarize(tachyon::json::JsonNode mode, const tachyon::json::JsonNode& detail) const;\n}};\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.hpp"),
                String::from(
                    "#pragma once\n\nclass [[RequestRoute(\"/report\")]] [[Public]] ReportController {\n    [[Needs]] ReportClient& reportClient;\n\npublic:\n    [[PostRoute]]\n    tachyon::json::JsonNode postReport([[FromBody]] tachyon::json::JsonNode input) {\n        try {\n            return reportClient.summarize(input[0], input[1]);\n        } catch (const tachyon::TacCodedError& error) {\n            tachyon::json::JsonObject failed;\n            failed[\"code\"] = error.code();\n            failed[\"message\"] = std::string(error.what());\n            return failed;\n        }\n    }\n};\n",
                ),
            ),
        ]
    }

    fn python(target: &str) -> [(String, String); 2] {
        [
            (
                String::from("server/remotes/ReportClient.py"),
                format!(
                    "@http_client\nclass ReportClient(Protocol):\n    @gateway({{\"class\":\"{target}\"}})\n    def summarize(self, mode, detail): ...\n"
                ),
            ),
            (
                String::from("server/controllers/ReportController.py"),
                String::from(
                    "import json\n\n\n@request_route(\"/report\")\n@public\nclass ReportController:\n    report_client: Annotated[ReportClient, needs]\n\n    @post_route\n    def post_report(self, request):\n        mode, detail = json.loads(request.data)\n        return self._summarized(mode, detail)\n\n    def _summarized(self, mode, detail):\n        try:\n            return self.report_client.summarize(mode, detail)\n        except BehaviorError as error:\n            return {\"code\": error.code, \"message\": str(error)}\n",
                ),
            ),
        ]
    }
}

fn available(program: &str) -> bool {
    let version = match program {
        "go" => "version",
        "lua" => "-v",
        _ => "--version",
    };
    StdCommand::new(program)
        .arg(version)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn usable(program: &str) -> bool {
    if available(program) {
        return true;
    }
    let required = cfg!(unix) && std::env::var_os("CI").is_some();
    assert!(
        !required,
        "{program} is required for the gateway tests in CI"
    );
    eprintln!("skipping: {program} is unavailable");
    false
}

fn kotlin_available() -> bool {
    let found = StdCommand::new("kotlinc")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    crate::support::toolchain::Toolchain::checked("kotlinc", found)
}

fn jvm_available(extension: &str) -> bool {
    if extension == "kt" {
        return kotlin_available();
    }
    available("javac")
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("file");
}

struct Project {
    root: tempfile::TempDir,
}

impl Project {
    fn new(files: &[(&str, &str)], caller: &[(String, String)]) -> Self {
        let root = tempfile::tempdir().expect("project");
        write(
            root.path(),
            "server/application.json",
            r#"{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}"#,
        );
        for (path, contents) in files {
            write(root.path(), path, contents);
        }
        for (path, contents) in caller {
            write(root.path(), path, contents);
        }
        Self { root }
    }

    fn entry(extension: &str) -> String {
        format!("server/controllers/ReportController.{extension}")
    }

    fn supervisor_with_bun(&self) -> HandlerSupervisor {
        let application = ApplicationConfig::load(self.root.path()).expect("application");
        let mut options = HandlerSupervisorOptions {
            application,
            ..HandlerSupervisorOptions::default()
        };
        options.runtimes.javascript = "bun".into();
        HandlerSupervisor::new(options).expect("supervisor")
    }

    fn discover(&self, extension: &str) -> HandlerSource {
        HandlerSource::discover(self.root.path(), Self::entry(extension)).expect("gateway caller")
    }

    fn failure(&self, extension: &str) -> String {
        HandlerSource::discover(self.root.path(), Self::entry(extension))
            .expect_err("refused gateway")
            .to_string()
    }

    fn supervisor(&self) -> HandlerSupervisor {
        let application = ApplicationConfig::load(self.root.path()).expect("application");
        HandlerSupervisor::new(HandlerSupervisorOptions {
            application,
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor")
    }
}

async fn call(
    project: &Project,
    source: &HandlerSource,
    arguments: &serde_json::Value,
) -> serde_json::Value {
    call_with(&project.supervisor(), source, arguments).await
}

async fn call_with(
    supervisor: &HandlerSupervisor,
    source: &HandlerSource,
    arguments: &serde_json::Value,
) -> serde_json::Value {
    let request = HandlerRequest::route("gateway_class", "/report", HttpMethod::Post);
    call_using(supervisor, source, arguments, request).await
}

async fn call_bound(
    project: &Project,
    source: &HandlerSource,
    arguments: &serde_json::Value,
) -> serde_json::Value {
    let mut request = HandlerRequest::route("gateway_class", "/report", HttpMethod::Post);
    request.arguments = Some(vec![arguments.clone()]);
    call_using(&project.supervisor(), source, arguments, request).await
}

async fn call_using(
    supervisor: &HandlerSupervisor,
    source: &HandlerSource,
    arguments: &serde_json::Value,
    mut request: HandlerRequest,
) -> serde_json::Value {
    request.headers.insert(
        String::from("authorization"),
        vec![String::from("Bearer caller-secret")],
    );
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: arguments.to_string(),
    });
    let response = supervisor
        .invoke(source, &request, &HandlerCancellation::default())
        .await
        .expect("caller response");
    let data = response.body.expect("body").data;
    assert!(!data.contains("secret-canary"), "{data}");
    assert!(!data.contains("caller-secret"), "{data}");
    serde_json::from_str(&data).expect("JSON body")
}

#[tokio::test]
async fn a_javascript_service_calls_a_python_class_with_its_own_data_service() {
    if !usable("node") || !usable("python3") {
        return;
    }
    let project = Project::new(
        &[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE],
        &Callers::javascript(Targets::PYTHON.0),
    );
    let source = project.discover("js");
    let detail = serde_json::json!({"period": "2026-Q3", "values": [1, 2]});
    let answer = call(&project, &source, &serde_json::json!(["store", detail])).await;
    assert_eq!(answer["language"], "python", "{answer}");
    assert_eq!(answer["detail"], detail, "{answer}");
    assert_eq!(answer["stored"], 1, "{answer}");
    let failed = call(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
    assert_eq!(failed["message"], "Gateway call failed.", "{failed}");
}

#[tokio::test]
async fn a_javascript_service_calls_a_ruby_class() {
    if !usable("node") || !usable("ruby") {
        return;
    }
    let project = Project::new(&[Targets::RUBY], &Callers::javascript(Targets::RUBY.0));
    let source = project.discover("js");
    let answer = call(&project, &source, &serde_json::json!(["plain", {"a": 1}])).await;
    assert_eq!(answer["language"], "ruby", "{answer}");
    assert_eq!(answer["detail"], serde_json::json!({"a": 1}), "{answer}");
    let failed = call(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
    assert_eq!(failed["message"], "Gateway call failed.", "{failed}");
}

#[tokio::test]
async fn a_python_service_calls_a_javascript_class() {
    if !usable("node") || !usable("python3") {
        return;
    }
    let project = Project::new(
        &[Targets::JAVASCRIPT],
        &Callers::python(Targets::JAVASCRIPT.0),
    );
    let source = project.discover("py");
    let answer = call(
        &project,
        &source,
        &serde_json::json!(["plain", [true, null]]),
    )
    .await;
    assert_eq!(answer["language"], "javascript", "{answer}");
    assert_eq!(
        answer["detail"],
        serde_json::json!([true, null]),
        "{answer}"
    );
    let failed = call(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
}

#[tokio::test]
async fn a_javascript_service_calls_a_php_class() {
    if !usable("node") || !available("php-cgi") {
        return;
    }
    let project = Project::new(&[Targets::PHP], &Callers::javascript(Targets::PHP.0));
    let source = project.discover("js");
    let answer = call(&project, &source, &serde_json::json!(["plain", {"a": 1}])).await;
    assert_eq!(answer["language"], "php", "{answer}");
    assert_eq!(answer["detail"], serde_json::json!({"a": 1}), "{answer}");
}

#[tokio::test]
async fn a_php_service_calls_a_python_class() {
    if !available("php-cgi") || !usable("python3") {
        return;
    }
    let project = Project::new(
        &[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE],
        &Callers::php(Targets::PYTHON.0),
    );
    let source = project.discover("php");
    let answer = call(&project, &source, &serde_json::json!(["store", "report"])).await;
    assert_eq!(answer["language"], "python", "{answer}");
    assert_eq!(answer["stored"], 1, "{answer}");
    let failed = call(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
    assert_eq!(failed["message"], "Gateway call failed.", "{failed}");
}

#[tokio::test]
async fn a_ruby_service_calls_a_python_class() {
    if !available("ruby") || !usable("python3") {
        return;
    }
    let project = Project::new(
        &[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE],
        &Callers::ruby(Targets::PYTHON.0),
    );
    let source = project.discover("rb");
    let answer = call(&project, &source, &serde_json::json!(["store", "report"])).await;
    assert_eq!(answer["language"], "python", "{answer}");
    assert_eq!(answer["stored"], 1, "{answer}");
    let failed = call(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
    assert_eq!(failed["message"], "Gateway call failed.", "{failed}");
}

#[tokio::test]
async fn a_lua_service_calls_a_python_class() {
    if !available("lua") || !usable("python3") {
        return;
    }
    let project = Project::new(
        &[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE],
        &Callers::lua(Targets::PYTHON.0),
    );
    let source = project.discover("lua");
    let answer = call_bound(&project, &source, &serde_json::json!(["store", "report"])).await;
    assert_eq!(answer["language"], "python", "{answer}");
    assert_eq!(answer["detail"], "report", "{answer}");
    assert_eq!(answer["stored"], 1, "{answer}");
    let failed = call_bound(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
    assert_eq!(failed["message"], "Gateway call failed.", "{failed}");
}

#[tokio::test]
async fn a_typescript_service_calls_a_php_class() {
    if !usable("bun") || !available("php-cgi") {
        return;
    }
    let project = Project::new(&[Targets::PHP], &Callers::typescript(Targets::PHP.0));
    let source = project.discover("ts");
    let supervisor = project.supervisor_with_bun();
    let answer = call_with(
        &supervisor,
        &source,
        &serde_json::json!(["plain", [1, "two"]]),
    )
    .await;
    assert_eq!(answer["language"], "php", "{answer}");
    assert_eq!(answer["detail"], serde_json::json!([1, "two"]), "{answer}");
    let failed = call_with(&supervisor, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
}

async fn assert_python_reaches_compiled_target(
    files: &[(&str, &str)],
    language: &str,
) -> serde_json::Value {
    let project = Project::new(files, &Callers::python(files[0].0));
    let source = project.discover("py");
    let detail = serde_json::json!({"period": "2026-Q3", "values": [1, 2.5, null, true]});
    let answer = call(&project, &source, &serde_json::json!(["plain", detail])).await;
    assert_eq!(answer["language"], language, "{answer}");
    assert_eq!(answer["mode"], "plain", "{answer}");
    assert_eq!(answer["detail"], detail, "{answer}");
    let nothing = call(&project, &source, &serde_json::json!(["nothing", null])).await;
    assert!(nothing.is_null(), "{nothing}");
    let failed = call(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
    assert_eq!(failed["message"], "Gateway call failed.", "{failed}");
    answer
}

#[tokio::test]
async fn a_python_service_calls_a_java_class_with_its_own_data_service() {
    if !usable("python3") || !jvm_available("java") {
        return;
    }
    let files = [Targets::JAVA, Targets::JAVA_DATA_SERVICE];
    let answer = assert_python_reaches_compiled_target(&files, "java").await;
    assert_eq!(answer["stored"][0]["total"], 1, "{answer}");
    assert_eq!(answer["shape"]["kind"], "report", "{answer}");
}

#[tokio::test]
async fn a_python_service_calls_a_kotlin_class() {
    if !usable("python3") || !jvm_available("kt") {
        return;
    }
    assert_python_reaches_compiled_target(&[Targets::KOTLIN], "kotlin").await;
}

async fn assert_compiled_caller_reaches_python(extension: &str, caller: &[(String, String)]) {
    let project = Project::new(&[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE], caller);
    let source = project.discover(extension);
    let detail = serde_json::json!({"period": "2026-Q3", "values": [1, 2.5]});
    let answer = call(&project, &source, &serde_json::json!(["store", detail])).await;
    assert_eq!(answer["language"], "python", "{extension}: {answer}");
    assert_eq!(answer["detail"], detail, "{extension}: {answer}");
    assert_eq!(answer["stored"], 1, "{extension}: {answer}");
    let failed = call(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{extension}: {failed}");
    assert_eq!(
        failed["message"], "Gateway call failed.",
        "{extension}: {failed}"
    );
}

#[tokio::test]
async fn a_java_service_calls_a_python_class() {
    if !usable("python3") || !jvm_available("java") {
        return;
    }
    assert_compiled_caller_reaches_python("java", &Callers::java(Targets::PYTHON.0)).await;
}

#[tokio::test]
async fn a_kotlin_service_calls_a_python_class() {
    if !usable("python3") || !jvm_available("kt") {
        return;
    }
    assert_compiled_caller_reaches_python("kt", &Callers::kotlin(Targets::PYTHON.0)).await;
}

#[test]
fn jvm_targets_must_use_the_json_value_type() {
    let java = "@Service\nclass TypedService {\n    Object summarize(String mode, Object detail) { return mode; }\n}\n";
    let kotlin = "@Service\nclass TypedService {\n    fun summarize(mode: String, detail: Any?): Any? = mode\n}\n";
    for (path, contents) in [
        ("server/services/TypedService.java", java),
        ("server/services/TypedService.kt", kotlin),
    ] {
        let project = Project::new(&[(path, contents)], &Callers::python(path));
        let failure = project.failure("py");
        assert!(failure.contains("TY1312"), "{path}: {failure}");
        assert!(failure.contains("JSON value type"), "{path}: {failure}");
    }
}

#[test]
fn jvm_callers_must_use_the_json_value_type() {
    let java = "@HTTPClient\ninterface ReportClient {\n    @Gateway({\"class\":\"server/services/ReportService.py\"})\n    Object summarize(String mode, Object detail);\n}\n";
    let kotlin = "@HTTPClient\ninterface ReportClient {\n    @Gateway({\"class\":\"server/services/ReportService.py\"})\n    fun summarize(mode: String, detail: Any?): Any?\n}\n";
    for (extension, contents) in [("java", java), ("kt", kotlin)] {
        let caller = Callers::with_client(extension, contents);
        let project = Project::new(&[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE], &caller);
        let failure = project.failure(extension);
        assert!(failure.contains("TY1312"), "{extension}: {failure}");
    }
}

#[test]
fn class_targets_are_checked_when_the_caller_is_built() {
    let wrong_arity =
        "@service\nclass CountService:\n    def summarize(self, mode):\n        return mode\n";
    let declares_gateway = "@service\nclass HopService:\n    def summarize(self, mode, detail):\n        return self.forward(mode)\n\n    @gateway({\"command\":[\"echo\",\"1\"]})\n    def forward(self, mode): ...\n";
    let static_method = "@service\nclass StaticService:\n    @staticmethod\n    def summarize(mode, detail):\n        return mode\n";
    let redefined = "@service\nclass RedefinedService:\n    def summarize(self, mode, detail):\n        return mode\n\n    def summarize(self, mode):\n        return mode\n";
    let cases = [
        (
            "server/services/MissingService.py",
            "not in the captured server snapshot",
        ),
        ("server/services/ReportService.js", "caller's own language"),
        ("server/services/Report.sh", "not written in a Yon language"),
        ("server/controllers/ReportController.py", "not a role file"),
        (
            "server/services/../services/ReportService.py",
            "not a role file",
        ),
        ("server/services/CountService.py", "takes 1 arguments"),
        ("server/services/HopService.py", "only one hop"),
        (
            "server/services/StaticService.py",
            "not a concrete instance method",
        ),
        ("server/services/RedefinedService.py", "takes 1 arguments"),
    ];
    for (target, expected) in cases {
        let project = Project::new(
            &[
                Targets::JAVASCRIPT,
                ("server/services/CountService.py", wrong_arity),
                ("server/services/HopService.py", declares_gateway),
                ("server/services/StaticService.py", static_method),
                ("server/services/RedefinedService.py", redefined),
                ("server/services/Report.sh", "echo 1\n"),
            ],
            &Callers::javascript(target),
        );
        let failure = project.failure("js");
        assert!(failure.contains("TY1312"), "{target}: {failure}");
        assert!(failure.contains(expected), "{target}: {failure}");
    }
}

#[tokio::test]
async fn a_python_service_calls_a_cpp_class() {
    if !usable("python3") || !usable("clang++") {
        return;
    }
    let files = [Targets::CPP, Targets::CPP_HELPER];
    assert_python_reaches_compiled_target(&files, "cpp").await;
}

#[tokio::test]
async fn a_cpp_service_calls_a_python_class() {
    if !usable("python3") || !usable("clang++") {
        return;
    }
    let project = Project::new(
        &[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE],
        &Callers::cpp(Targets::PYTHON.0),
    );
    let source = project.discover("hpp");
    let detail = serde_json::json!({"period": "2026-Q3", "values": [1, 2.5]});
    let answer = call_bound(&project, &source, &serde_json::json!(["store", detail])).await;
    assert_eq!(answer["language"], "python", "{answer}");
    assert_eq!(answer["detail"], detail, "{answer}");
    assert_eq!(answer["stored"], 1, "{answer}");
    let failed = call_bound(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{failed}");
    assert_eq!(failed["message"], "Gateway call failed.", "{failed}");
}

#[test]
fn cpp_gateway_targets_are_public_json_instance_methods() {
    let summarize = "tachyon::json::JsonNode summarize(tachyon::json::JsonNode mode, const tachyon::json::JsonNode& detail) {";
    let cases = [
        (
            summarize,
            "std::string summarize(tachyon::json::JsonNode mode, const tachyon::json::JsonNode& detail) {",
            "JSON value type",
        ),
        (
            summarize,
            "static tachyon::json::JsonNode summarize(tachyon::json::JsonNode mode, const tachyon::json::JsonNode& detail) {",
            "not a concrete instance method",
        ),
        (
            "public:\n    tachyon::json::JsonNode summarize(",
            "    tachyon::json::JsonNode summarize(",
            "not a concrete instance method",
        ),
        (
            summarize,
            "tachyon::json::JsonNode summarize(tachyon::json::JsonNode mode) {",
            "takes 1 arguments",
        ),
    ];
    for (from, to, expected) in cases {
        assert!(Targets::CPP.1.contains(from), "{from}");
        let target = Targets::CPP.1.replace(from, to);
        let project = Project::new(
            &[(Targets::CPP.0, target.as_str()), Targets::CPP_HELPER],
            &Callers::python(Targets::CPP.0),
        );
        let failure = project.failure("py");
        assert!(failure.contains("TY1312"), "{to}: {failure}");
        assert!(failure.contains(expected), "{to}: {failure}");
    }
}

#[test]
fn rust_gateway_targets_require_shared_receivers() {
    for receiver in ["self", "&mut self"] {
        let target = Targets::RUST
            .1
            .replace("&self, mode", &format!("{receiver}, mode"));
        let project = Project::new(
            &[(Targets::RUST.0, &target)],
            &Callers::javascript(Targets::RUST.0),
        );
        let failure = project.failure("js");
        assert!(failure.contains("TY1312"), "{receiver}: {failure}");
        assert!(failure.contains("&self"), "{receiver}: {failure}");
    }
}

#[test]
fn gateway_overloads_select_the_matching_arity_and_json_types() {
    if !jvm_available("java") {
        return;
    }
    let methods = [
        "    String summarize(String mode, String detail) { return mode; }\n",
        "    Object summarize(Object mode) { return mode; }\n",
        "    Object summarize(Object mode, Object detail) { return detail; }\n",
    ];
    for order in [[0, 1, 2], [2, 1, 0], [1, 2, 0]] {
        let mut target = String::from("@Service\nclass ReportService {\n");
        for index in order {
            target.push_str(methods[index]);
        }
        target.push_str("}\n");
        let project = Project::new(
            &[(Targets::JAVA.0, &target)],
            &Callers::javascript(Targets::JAVA.0),
        );
        project.discover("js");
    }
}

#[tokio::test]
async fn a_gateway_calls_the_selected_java_overload() {
    if !usable("python3") || !jvm_available("java") {
        return;
    }
    let target = "@Service\nclass ReportService {\n    String summarize(String mode, String detail) { return mode; }\n    Object summarize(Object mode) { return mode; }\n    Object summarize(Object mode, Object detail) { return detail; }\n}\n";
    let project = Project::new(
        &[(Targets::JAVA.0, target)],
        &Callers::python(Targets::JAVA.0),
    );
    let source = project.discover("py");
    let detail = serde_json::json!({"selected": "two JSON arguments"});
    let answer = call(&project, &source, &serde_json::json!(["mode", detail])).await;
    assert_eq!(answer, detail);
}

#[test]
fn ambiguous_gateway_target_signatures_are_refused() {
    let target = "@Service\nclass ReportService {\n    Object summarize(Object mode, Object detail) { return mode; }\n    Object summarize(Object mode, Object detail) { return detail; }\n}\n";
    let project = Project::new(
        &[(Targets::JAVA.0, target)],
        &Callers::javascript(Targets::JAVA.0),
    );
    let failure = project.failure("js");
    assert!(failure.contains("TY1312"), "{failure}");
    assert!(failure.contains("ambiguous"), "{failure}");
}

#[test]
fn gateway_callers_cannot_select_different_arities_of_one_target_method() {
    let target = "@Service\nclass ReportService {\n    Object summarize(Object mode) { return mode; }\n    Object summarize(Object mode, Object detail) { return detail; }\n}\n";
    for order in [[1, 2], [2, 1]] {
        let mut caller = Callers::javascript(Targets::JAVA.0);
        let mut client = String::from("@HTTPClient\nclass ReportClient {\n");
        for arity in order {
            let parameters = if arity == 1 { "mode" } else { "mode, detail" };
            write!(
                client,
                "  @Gateway({{\"class\":\"{}\",\"method\":\"summarize\"}})\n  async summarize{arity}({parameters}) {{}}\n",
                Targets::JAVA.0,
            )
            .expect("gateway declaration");
        }
        client.push_str("}\n");
        caller[0].1 = client;
        let project = Project::new(&[(Targets::JAVA.0, target)], &caller);
        let failure = project.failure("js");
        assert!(failure.contains("TY1312"), "{failure}");
        assert!(failure.contains("different argument counts"), "{failure}");
    }
}

#[test]
fn authored_files_under_server_gateways_are_refused() {
    for authored in [
        "server/gateways/manual/report.js",
        "server/Gateways/manual/report.js",
        "server/GATEWAYS/manual/report.js",
    ] {
        let project = Project::new(
            &[Targets::JAVASCRIPT, (authored, "export class Handler {}\n")],
            &Callers::python(Targets::JAVASCRIPT.0),
        );
        let failure = project.failure("py");
        assert!(failure.contains("TY1310"), "{authored}: {failure}");
        assert!(
            failure.contains("reserves for generated gateway units"),
            "{authored}: {failure}"
        );
    }
}

#[tokio::test]
async fn a_python_service_calls_a_csharp_class() {
    if !usable("python3") || !available("dotnet") {
        return;
    }
    assert_python_reaches_compiled_target(&[Targets::CSHARP], "csharp").await;
}

#[tokio::test]
async fn a_csharp_service_calls_a_python_class() {
    if !usable("python3") || !available("dotnet") {
        return;
    }
    assert_compiled_caller_reaches_python("cs", &Callers::csharp(Targets::PYTHON.0)).await;
}

#[tokio::test]
async fn a_python_service_calls_a_rust_class() {
    if !usable("python3") || !available("rustc") {
        return;
    }
    assert_python_reaches_compiled_target(&[Targets::RUST], "rust").await;
}

#[tokio::test]
async fn a_rust_service_calls_a_python_class() {
    if !usable("python3") || !available("rustc") {
        return;
    }
    assert_compiled_caller_reaches_python("rs", &Callers::rust(Targets::PYTHON.0)).await;
}

#[tokio::test]
async fn a_go_service_calls_a_python_class() {
    if !usable("python3") || !usable("go") {
        return;
    }
    assert_compiled_caller_reaches_python("go", &Callers::go(Targets::PYTHON.0)).await;
}

#[tokio::test]
async fn a_python_service_calls_a_dart_class() {
    if !usable("python3") || !available("dart") {
        return;
    }
    assert_python_reaches_compiled_target(&[Targets::DART], "dart").await;
}

#[tokio::test]
async fn a_dart_service_calls_a_python_class() {
    if !usable("python3") || !available("dart") {
        return;
    }
    assert_compiled_caller_reaches_python("dart", &Callers::dart(Targets::PYTHON.0)).await;
}

async fn assert_caller_reaches_go(extension: &str, caller: &[(String, String)]) {
    let project = Project::new(&[Targets::GO, Targets::GO_DATA_SERVICE], caller);
    let source = project.discover(extension);
    let supervisor = if extension == "ts" {
        project.supervisor_with_bun()
    } else {
        project.supervisor()
    };
    let detail = serde_json::json!({"period": "2026-Q3", "values": [1, 2.5, null, true]});
    let answer = call_with(&supervisor, &source, &serde_json::json!(["store", detail])).await;
    assert_eq!(answer["language"], "go", "{extension}: {answer}");
    assert_eq!(answer["mode"], "store", "{extension}: {answer}");
    assert_eq!(answer["detail"], detail, "{extension}: {answer}");
    assert_eq!(answer["stored"][0]["total"], 1, "{extension}: {answer}");
    let failed = call_with(&supervisor, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{extension}: {failed}");
}

#[tokio::test]
async fn a_javascript_service_calls_a_go_class_with_its_own_data_service() {
    if !usable("node") || !usable("go") {
        return;
    }
    assert_caller_reaches_go("js", &Callers::javascript(Targets::GO.0)).await;
}

#[tokio::test]
async fn a_typescript_service_calls_a_go_class() {
    if !usable("bun") || !usable("go") {
        return;
    }
    assert_caller_reaches_go("ts", &Callers::typescript(Targets::GO.0)).await;
}

#[tokio::test]
async fn a_python_service_calls_a_go_class() {
    if !usable("python3") || !usable("go") {
        return;
    }
    let caller = Callers::python(Targets::GO.0);
    assert_caller_reaches_go("py", &caller).await;
    let project = Project::new(&[Targets::GO, Targets::GO_DATA_SERVICE], &caller);
    let source = project.discover("py");
    let nothing = call(&project, &source, &serde_json::json!(["nothing", null])).await;
    assert!(nothing.is_null(), "{nothing}");
}

#[tokio::test]
async fn a_php_service_calls_a_go_class() {
    if !available("php-cgi") || !usable("go") {
        return;
    }
    assert_caller_reaches_go("php", &Callers::php(Targets::GO.0)).await;
}

#[tokio::test]
async fn a_java_service_calls_a_go_class() {
    if !jvm_available("java") || !usable("go") {
        return;
    }
    assert_caller_reaches_go("java", &Callers::java(Targets::GO.0)).await;
}

#[tokio::test]
async fn a_kotlin_service_calls_a_go_class() {
    if !jvm_available("kt") || !usable("go") {
        return;
    }
    assert_caller_reaches_go("kt", &Callers::kotlin(Targets::GO.0)).await;
}

#[tokio::test]
async fn a_csharp_service_calls_a_go_class() {
    if !available("dotnet") || !usable("go") {
        return;
    }
    assert_caller_reaches_go("cs", &Callers::csharp(Targets::GO.0)).await;
}

#[tokio::test]
async fn a_rust_service_calls_a_go_class() {
    if !available("rustc") || !usable("go") {
        return;
    }
    assert_caller_reaches_go("rs", &Callers::rust(Targets::GO.0)).await;
}

#[tokio::test]
async fn a_dart_service_calls_a_go_class() {
    if !available("dart") || !usable("go") {
        return;
    }
    assert_caller_reaches_go("dart", &Callers::dart(Targets::GO.0)).await;
}

async fn assert_go_caller_reaches(files: &[(&str, &str)], language: &str) -> serde_json::Value {
    let project = Project::new(files, &Callers::go(files[0].0));
    let source = project.discover("go");
    let detail = serde_json::json!({"period": "2026-Q3", "values": [1, 2.5, null, true]});
    let answer = call(&project, &source, &serde_json::json!(["plain", detail])).await;
    assert_eq!(answer["language"], language, "{answer}");
    assert_eq!(answer["detail"], detail, "{answer}");
    let failed = call(&project, &source, &serde_json::json!(["fail", null])).await;
    assert_eq!(failed["code"], "adapter", "{language}: {failed}");
    assert_eq!(
        failed["message"], "Gateway call failed.",
        "{language}: {failed}"
    );
    answer
}

#[tokio::test]
async fn a_go_service_calls_a_javascript_class() {
    if !usable("node") || !usable("go") {
        return;
    }
    assert_go_caller_reaches(&[Targets::JAVASCRIPT], "javascript").await;
}

#[tokio::test]
async fn a_go_service_calls_a_php_class() {
    if !available("php-cgi") || !usable("go") {
        return;
    }
    assert_go_caller_reaches(&[Targets::PHP], "php").await;
}

#[tokio::test]
async fn a_go_service_calls_a_java_class_with_its_own_data_service() {
    if !jvm_available("java") || !usable("go") {
        return;
    }
    let files = [Targets::JAVA, Targets::JAVA_DATA_SERVICE];
    let answer = assert_go_caller_reaches(&files, "java").await;
    assert_eq!(answer["stored"][0]["total"], 1, "{answer}");
}

#[tokio::test]
async fn a_go_service_calls_a_kotlin_class() {
    if !jvm_available("kt") || !usable("go") {
        return;
    }
    assert_go_caller_reaches(&[Targets::KOTLIN], "kotlin").await;
}

#[tokio::test]
async fn a_go_service_calls_a_csharp_class() {
    if !available("dotnet") || !usable("go") {
        return;
    }
    assert_go_caller_reaches(&[Targets::CSHARP], "csharp").await;
}

#[tokio::test]
async fn a_go_service_calls_a_cpp_class() {
    if !usable("clang++") || !usable("go") {
        return;
    }
    assert_go_caller_reaches(&[Targets::CPP, Targets::CPP_HELPER], "cpp").await;
}

#[tokio::test]
async fn a_go_service_calls_a_rust_class() {
    if !available("rustc") || !usable("go") {
        return;
    }
    assert_go_caller_reaches(&[Targets::RUST], "rust").await;
}

#[tokio::test]
async fn a_go_service_calls_a_dart_class() {
    if !available("dart") || !usable("go") {
        return;
    }
    assert_go_caller_reaches(&[Targets::DART], "dart").await;
}

#[tokio::test]
async fn go_class_targets_accept_value_and_pointer_receivers() {
    if !usable("python3") || !usable("go") {
        return;
    }
    let value_receiver = Targets::GO.1.replace(
        "func (service *ReportService)",
        "func (service ReportService)",
    );
    let project = Project::new(
        &[(Targets::GO.0, &value_receiver), Targets::GO_DATA_SERVICE],
        &Callers::python(Targets::GO.0),
    );
    let source = project.discover("py");
    let answer = call(&project, &source, &serde_json::json!(["store", "value"])).await;
    assert_eq!(answer["language"], "go", "{answer}");
    assert_eq!(answer["stored"][0]["total"], 1, "{answer}");
}

struct GoCheckedTargets;

impl GoCheckedTargets {
    const UNEXPORTED: &str = "// @Service\ntype LowerService struct{}\n\nfunc (service *LowerService) summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {\n\treturn mode, nil\n}\n";

    const ONE_ARGUMENT: &str = "// @Service\ntype CountService struct{}\n\nfunc (service *CountService) Summarize(mode BehaviorValue) (BehaviorValue, error) {\n\treturn mode, nil\n}\n";

    const TWICE: &str = "// @Service\ntype TwiceService struct{}\n\nfunc (service *TwiceService) Summarize(mode BehaviorValue, detail BehaviorValue) BehaviorValue {\n\treturn mode\n}\n\nfunc (service TwiceService) Summarize(mode BehaviorValue, detail BehaviorValue) BehaviorValue {\n\treturn detail\n}\n";

    const NAMED_RESULTS: &str = "// @Service\ntype NamedService struct{}\n\nfunc (service *NamedService) Summarize(mode BehaviorValue, detail BehaviorValue) (value BehaviorValue, failure error) {\n\treturn mode, nil\n}\n";

    const DECLARES_GATEWAY: &str = "// @Service\ntype HopService struct{}\n\nfunc (service *HopService) Summarize(mode BehaviorValue, detail BehaviorValue) (BehaviorValue, error) {\n\treturn service.Forward(mode)\n}\n\n// @Gateway({\"command\":[\"echo\",\"1\"]})\nfunc (service *HopService) Forward(mode BehaviorValue) (BehaviorValue, error) {}\n";
}

#[test]
fn go_class_targets_are_checked_when_the_caller_is_built() {
    let cases = [
        (
            "server/services/LowerService.go",
            "exported Go method Summarize",
        ),
        ("server/services/CountService.go", "takes 1 arguments"),
        ("server/services/TwiceService.go", "ambiguous"),
        ("server/services/NamedService.go", "JSON value type"),
        ("server/services/HopService.go", "only one hop"),
        (
            "server/services/MissingService.go",
            "not in the captured server snapshot",
        ),
    ];
    for (target, expected) in cases {
        let project = Project::new(
            &[
                (
                    "server/services/LowerService.go",
                    GoCheckedTargets::UNEXPORTED,
                ),
                (
                    "server/services/CountService.go",
                    GoCheckedTargets::ONE_ARGUMENT,
                ),
                ("server/services/TwiceService.go", GoCheckedTargets::TWICE),
                (
                    "server/services/NamedService.go",
                    GoCheckedTargets::NAMED_RESULTS,
                ),
                (
                    "server/services/HopService.go",
                    GoCheckedTargets::DECLARES_GATEWAY,
                ),
            ],
            &Callers::javascript(target),
        );
        let failure = project.failure("js");
        assert!(failure.contains("TY1312"), "{target}: {failure}");
        assert!(failure.contains(expected), "{target}: {failure}");
    }
}

struct TypedTargets;

impl TypedTargets {
    const NATIVE: [(&str, &str); 4] = [
        (
            "server/services/TypedService.dart",
            "@Service\nclass TypedService {\n  Future<Object?> summarize(String mode, Object? detail) async => detail;\n}\n",
        ),
        (
            "server/services/TypedService.cs",
            "[Service]\nclass TypedService {\n    public BehaviorJSON summarize(string mode, BehaviorJSON detail) { return detail; }\n}\n",
        ),
        (
            "server/services/TypedService.rs",
            "#[yon::service]\nstruct TypedService;\nimpl TypedService {\n    fn new() -> Self { Self }\n    fn summarize(&self, mode: String, detail: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(detail) }\n}\n",
        ),
        (
            "server/services/TypedService.go",
            "// @Service\ntype TypedService struct{}\n\nfunc (service *TypedService) Summarize(mode string, detail BehaviorValue) (BehaviorValue, error) {\n\treturn detail, nil\n}\n",
        ),
    ];

    const NATIVE_CALLERS: [(&str, &str); 4] = [
        (
            "dart",
            "@HTTPClient\nabstract interface class ReportClient {\n  @Gateway({\"class\":\"server/services/ReportService.py\"})\n  Future<Object?> summarize(String mode, Object? detail);\n}\n",
        ),
        (
            "cs",
            "[HTTPClient]\npublic interface ReportClient {\n    [Gateway({\"class\":\"server/services/ReportService.py\"})]\n    BehaviorJSON summarize(string mode, BehaviorJSON detail);\n}\n",
        ),
        (
            "rs",
            "#[HTTPClient]\ntrait ReportClient {\n    #[Gateway({\"class\":\"server/services/ReportService.py\"})]\n    fn summarize(&self, mode: String, detail: BehaviorValue) -> BehaviorResult<BehaviorValue>;\n}\n",
        ),
        (
            "go",
            "// @HTTPClient\ntype ReportClient interface {\n\t// @Gateway({\"class\":\"server/services/ReportService.py\",\"method\":\"summarize\"})\n\tSummarize(mode string, detail BehaviorValue) (BehaviorValue, error)\n}\n",
        ),
    ];
}

#[test]
fn native_targets_must_use_the_json_value_type() {
    for (path, contents) in TypedTargets::NATIVE {
        let project = Project::new(&[(path, contents)], &Callers::python(path));
        let failure = project.failure("py");
        assert!(failure.contains("TY1312"), "{path}: {failure}");
        assert!(failure.contains("JSON value type"), "{path}: {failure}");
    }
}

#[test]
fn native_callers_must_use_the_json_value_type() {
    for (extension, contents) in TypedTargets::NATIVE_CALLERS {
        let caller = Callers::with_client(extension, contents);
        let project = Project::new(&[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE], &caller);
        let failure = project.failure(extension);
        assert!(failure.contains("TY1312"), "{extension}: {failure}");
    }
}

fn python_target_project() -> (Project, HandlerSource) {
    let project = Project::new(
        &[Targets::PYTHON, Targets::PYTHON_DATA_SERVICE],
        &Callers::javascript(Targets::PYTHON.0),
    );
    let source = project.discover("js");
    (project, source)
}

fn go_target_project() -> (Project, HandlerSource) {
    let project = Project::new(
        &[Targets::GO, Targets::GO_DATA_SERVICE],
        &Callers::python(Targets::GO.0),
    );
    let source = project.discover("py");
    (project, source)
}

fn supervisor_with(project: &Project, options: HandlerSupervisorOptions) -> HandlerSupervisor {
    let application = ApplicationConfig::load(project.root.path()).expect("application");
    HandlerSupervisor::new(HandlerSupervisorOptions {
        application,
        ..options
    })
    .expect("supervisor")
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    StdCommand::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
async fn wait_for_pid(pid_file: &Path) -> u32 {
    for _attempt in 0..500 {
        if let Ok(text) = fs::read_to_string(pid_file)
            && let Ok(pid) = text.trim().parse()
        {
            return pid;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("the class target did not start its descendant");
}

#[cfg(unix)]
async fn assert_process_gone(pid: u32) {
    for _attempt in 0..100 {
        if !process_exists(pid) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("class target descendant {pid} survived");
}

async fn assert_unit_runs_while_the_handler_pool_is_full(
    project: &Project,
    source: &HandlerSource,
    language: &str,
) {
    let options = HandlerSupervisorOptions {
        max_concurrency: 1,
        default_timeout: std::time::Duration::from_secs(20),
        ..HandlerSupervisorOptions::default()
    };
    let supervisor = supervisor_with(project, options);
    let answer = call_with(&supervisor, source, &serde_json::json!(["store", 1])).await;
    assert_eq!(answer["language"], language, "{answer}");
}

#[tokio::test]
async fn class_target_units_run_while_the_handler_pool_is_full() {
    if !usable("node") || !usable("python3") {
        return;
    }
    let (project, source) = python_target_project();
    assert_unit_runs_while_the_handler_pool_is_full(&project, &source, "python").await;
}

#[tokio::test]
async fn go_class_target_units_run_while_the_handler_pool_is_full() {
    if !usable("python3") || !usable("go") {
        return;
    }
    let (project, source) = go_target_project();
    assert_unit_runs_while_the_handler_pool_is_full(&project, &source, "go").await;
}

#[cfg(unix)]
async fn assert_timeout_reaps_the_unit(project: &Project, source: &HandlerSource) {
    let options = HandlerSupervisorOptions {
        default_timeout: std::time::Duration::from_secs(6),
        cancellation_grace: std::time::Duration::from_millis(500),
        ..HandlerSupervisorOptions::default()
    };
    let supervisor = supervisor_with(project, options);
    let pid_file = project.root.path().join("hang.pid");
    let arguments = serde_json::json!(["hang", pid_file.to_string_lossy()]);
    let answer = call_with(&supervisor, source, &arguments).await;
    assert_eq!(answer["code"], "timeout", "{answer}");
    assert_process_gone(wait_for_pid(&pid_file).await).await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_class_target_timeout_reaps_the_unit_and_its_descendants() {
    if !usable("node") || !usable("python3") {
        return;
    }
    let (project, source) = python_target_project();
    assert_timeout_reaps_the_unit(&project, &source).await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_go_class_target_timeout_reaps_the_unit_and_its_descendants() {
    if !usable("python3") || !usable("go") {
        return;
    }
    let (project, source) = go_target_project();
    assert_timeout_reaps_the_unit(&project, &source).await;
}

#[cfg(unix)]
async fn assert_cancelling_the_caller_reaps_the_unit(project: &Project, source: &HandlerSource) {
    let supervisor = project.supervisor();
    let pid_file = project.root.path().join("cancel.pid");
    let mut request = HandlerRequest::route("gateway_cancel", "/report", HttpMethod::Post);
    request.body = Some(HandlerBody {
        encoding: HandlerBodyEncoding::Utf8,
        data: serde_json::json!(["hang", pid_file.to_string_lossy()]).to_string(),
    });
    let cancellation = HandlerCancellation::default();
    let canceller = tokio::spawn({
        let cancellation = cancellation.clone();
        let pid_file = pid_file.clone();
        async move {
            let pid = wait_for_pid(&pid_file).await;
            cancellation.cancel();
            pid
        }
    });
    let failure = supervisor
        .invoke(source, &request, &cancellation)
        .await
        .expect_err("cancelled caller");
    assert!(failure.to_string().contains("TY2111"), "{failure}");
    let pid = canceller.await.expect("canceller");
    assert_process_gone(pid).await;
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_the_caller_reaps_a_class_target_unit() {
    if !usable("node") || !usable("python3") {
        return;
    }
    let (project, source) = python_target_project();
    assert_cancelling_the_caller_reaps_the_unit(&project, &source).await;
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_the_caller_reaps_a_go_class_target_unit() {
    if !usable("python3") || !usable("go") {
        return;
    }
    let (project, source) = go_target_project();
    assert_cancelling_the_caller_reaps_the_unit(&project, &source).await;
}

#[tokio::test]
async fn a_unit_reaches_no_further_gateway_at_run_time() {
    if !usable("node") || !usable("python3") {
        return;
    }
    let (project, source) = python_target_project();
    let answer = call(&project, &source, &serde_json::json!(["hop", null])).await;
    assert_eq!(answer, serde_json::json!({"error": "adapter"}));
}

#[tokio::test]
async fn a_go_unit_reaches_no_further_gateway_at_run_time() {
    if !usable("python3") || !usable("go") {
        return;
    }
    let (project, source) = go_target_project();
    let answer = call(&project, &source, &serde_json::json!(["hop", null])).await;
    assert_eq!(answer, serde_json::json!("{\"error\":\"adapter\"}"));
}
