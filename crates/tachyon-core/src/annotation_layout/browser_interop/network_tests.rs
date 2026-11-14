use super::BrowserInteropCheck;
use std::path::Path;

fn codes(relative: &str, source: &str) -> Vec<String> {
    BrowserInteropCheck::problems(Path::new(relative), source)
        .into_iter()
        .map(|problem| problem.code.to_string())
        .collect()
}

fn messages(relative: &str, source: &str) -> Vec<String> {
    BrowserInteropCheck::problems(Path::new(relative), source)
        .into_iter()
        .map(|problem| problem.message)
        .collect()
}

#[test]
fn a_rust_service_that_uses_reqwest_is_refused() {
    let source = "#[Service]\npub struct PlaceService;\n\nimpl PlaceService {\n    pub async fn read(&self) {\n        let _ = reqwest::get(\"/api/places\").await;\n    }\n}\n";
    assert_eq!(
        codes("client/services/place_service.rs", source),
        ["TY1361"]
    );
    assert_eq!(
        messages("client/services/place_service.rs", source),
        [
            "PlaceService calls the network itself with `reqwest` in 'client/services/place_service.rs' line 6. Tac code reaches the network only through a remote: an HTTPClient in client/remotes."
        ]
    );
}

#[test]
fn a_csharp_service_that_uses_http_client_is_refused() {
    let source = "using System.Net.Http;\n\n[Service]\npublic class PlaceService {\n    private readonly HttpClient http = new();\n}\n";
    assert_eq!(
        codes("client/services/PlaceService.cs", source),
        ["TY1361", "TY1361"]
    );
}

#[test]
fn an_fsharp_service_that_uses_web_client_is_refused() {
    let source = "open System.Net\n\n[<Service>]\ntype PlaceService() =\n    member _.Read() = (new WebClient()).DownloadString(\"/api\")\n";
    assert_eq!(codes("client/services/PlaceService.fs", source), ["TY1361"]);
}

#[test]
fn a_dart_service_that_imports_package_http_is_refused() {
    let source = "import 'package:http/http.dart' as http;\n\n@Service()\nclass PlaceService {}\n";
    assert_eq!(
        codes("client/services/place_service.dart", source),
        ["TY1361"]
    );
}

#[test]
fn a_kotlin_service_that_uses_ktor_is_refused() {
    let source = "import io.ktor.client.HttpClient\n\n@Service\nclass PlaceService\n";
    assert_eq!(
        codes("client/services/PlaceService.kt", source),
        ["TY1361", "TY1361"]
    );
}

#[test]
fn a_swift_service_that_uses_url_session_is_refused() {
    let source = "@Service\nfinal class PlaceService {\n    func read() async throws {\n        _ = try await URLSession.shared.data(from: url)\n    }\n}\n";
    assert_eq!(
        codes("client/services/PlaceService.swift", source),
        ["TY1361"]
    );
}

#[test]
fn a_java_service_that_uses_http_url_connection_is_refused() {
    let source = "import java.net.HttpURLConnection;\n\n@Service\npublic class PlaceService {}\n";
    assert_eq!(
        codes("client/services/PlaceService.java", source),
        ["TY1361"]
    );
}

#[test]
fn a_cpp_service_that_calls_emscripten_fetch_is_refused() {
    let source = "class [[Service]] PlaceService {\n  void read() { emscripten_fetch(&attributes, \"/api\"); }\n};\n";
    assert_eq!(
        codes("client/services/PlaceService.hpp", source),
        ["TY1361"]
    );
}

#[test]
fn a_cpp_fetch_header_is_one_network_problem() {
    let source = "#include <emscripten/fetch.h>\n";
    assert_eq!(
        codes("client/services/PlaceService.hpp", source),
        ["TY1361"]
    );
}

#[test]
fn a_ruby_service_that_uses_net_http_is_refused() {
    let source = "require \"net/http\"\n\n# @Service\nclass PlaceService\n  def read = Net::HTTP.get(URI(\"/api\"))\nend\n";
    assert_eq!(
        codes("client/services/place_service.rb", source),
        ["TY1361", "TY1361"]
    );
}

#[test]
fn a_python_service_that_imports_fetch_from_js_is_one_network_problem() {
    let source = "from js import fetch\n\n@service\nclass PlaceService:\n    pass\n";
    assert_eq!(
        codes("client/services/place_service.py", source),
        ["TY1361"]
    );
}

#[test]
fn a_python_service_that_uses_pyodide_http_is_refused() {
    let source = "from pyodide.http import pyfetch\n\n@service\nclass PlaceService:\n    async def read(self):\n        return await pyfetch(\"/api\")\n";
    assert_eq!(
        codes("client/services/place_service.py", source),
        ["TY1361", "TY1361", "TY1361"]
    );
}

#[test]
fn a_python_service_that_imports_requests_is_refused() {
    let source = "import requests\n\n@service\nclass PlaceService:\n    pass\n";
    assert_eq!(
        codes("client/services/place_service.py", source),
        ["TY1361"]
    );
}

#[test]
fn a_python_service_that_imports_the_js_module_is_a_browser_problem() {
    let source = "from js import document\n\n@service\nclass PlaceService:\n    pass\n";
    assert_eq!(
        codes("client/services/place_service.py", source),
        ["TY1317"]
    );
}

#[test]
fn python_names_that_only_look_like_network_modules_are_not_refused() {
    let source = "import json\nfrom jsonschema import validate\n\n@service\nclass PlaceService:\n    requests = []\n    # import requests\n    label = \"from js import fetch\"\n";
    assert!(codes("client/services/place_service.py", source).is_empty());
}

#[test]
fn a_remote_named_like_a_network_library_is_not_refused() {
    let source =
        "[Service]\npublic class PlaceService {\n    [Needs] private PlacesHttpClient places;\n}\n";
    assert!(codes("client/services/PlaceService.cs", source).is_empty());
}

#[test]
fn server_code_may_use_the_network_libraries() {
    let source = "using System.Net.Http;\n";
    assert!(codes("server/services/PlaceService.cs", source).is_empty());
}
