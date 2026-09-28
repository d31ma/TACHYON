use super::{DeclaredHosts, HostCheck};
use std::path::Path;

fn refusals(path: &str, source: &str, declared: &[&str]) -> Vec<String> {
    let tac = DeclaredHosts::listed(declared, "tac.outgoing.hosts");
    let yon = DeclaredHosts::listed(declared, "outgoing.hosts");
    HostCheck::new(&tac, &yon)
        .diagnostics([(Path::new(path), source.as_bytes())].into_iter())
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

#[test]
fn a_request_to_an_undeclared_host_is_refused_in_every_language() {
    for (path, source) in [
        (
            "client/services/A.js",
            "load() { return fetch('https://api.example.com/items') }\n",
        ),
        (
            "server/services/A.ts",
            "load() { return fetch(`https://api.example.com/items/${id}`) }\n",
        ),
        (
            "server/services/A.py",
            "def load(self):\n    return urlopen(\"https://api.example.com/items\")\n",
        ),
        (
            "server/services/A.py",
            "def load(self):\n    return requests.get(\"https://api.example.com/items\")\n",
        ),
        (
            "server/services/A.rb",
            "def load\n  Net::HTTP.get(URI(\"https://api.example.com/items\"))\nend\n",
        ),
        (
            "server/services/A.php",
            "<?php\nfunction load() { return file_get_contents('https://api.example.com/items'); }\n",
        ),
        (
            "server/services/A.java",
            "Object load() { return HttpRequest.newBuilder(URI.create(\"https://api.example.com/items\")).build(); }\n",
        ),
        (
            "server/services/A.kt",
            "fun load() = URL(\"https://api.example.com/items\").readText()\n",
        ),
        (
            "server/services/A.cs",
            "Task<string> Load() => client.GetStringAsync(\"https://api.example.com/items\");\n",
        ),
        (
            "server/services/A.rs",
            "fn load() { reqwest::get(\"https://api.example.com/items\"); }\n",
        ),
        (
            "server/services/A.dart",
            "Future<Response> load() => get(Uri.parse('https://api.example.com/items'));\n",
        ),
        (
            "server/services/A.go",
            "func load() { http.Get(\"https://api.example.com/items\") }\n",
        ),
        (
            "server/services/A.go",
            "func load() { http.NewRequest(\"GET\", \"https://api.example.com/items\", nil) }\n",
        ),
    ] {
        let found = refusals(path, source, &["other.example.com"]);
        assert_eq!(found.len(), 1, "{path}: {source}");
        assert!(found[0].contains("'api.example.com'"), "{found:?}");
    }
}

#[test]
fn declared_hosts_and_links_are_allowed() {
    for (path, source) in [
        (
            "server/services/A.js",
            "load() { return fetch('https://api.example.com/items') }\n",
        ),
        (
            "server/services/A.js",
            "load() { return fetch('https://eu.cdn.example.com/items') }\n",
        ),
        (
            "client/services/A.js",
            "link() { return 'https://github.com/d31ma/tachyon' }\n",
        ),
        (
            "client/services/A.rs",
            "fn link() -> &'static str { \"https://github.com\" }\n",
        ),
        (
            "server/services/A.js",
            "load() { return fetch('/api/items') }\n",
        ),
        (
            "client/shared/scripts/a.js",
            "const u = new URL(h, \"https://base.invalid/\");\n",
        ),
        (
            "client/shared/data/docs.json",
            "{\"code\": \"fetch('https://api.other.com/')\"}\n",
        ),
    ] {
        let found = refusals(path, source, &["api.example.com", "*.cdn.example.com"]);
        assert!(found.is_empty(), "{path}: {found:?}");
    }
}

#[test]
fn a_host_built_while_the_code_runs_is_refused() {
    let found = refusals(
        "server/services/A.js",
        "load(host) { return fetch(`https://${host}/items`) }\n",
        &["api.example.com"],
    );
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("builds while it runs"), "{found:?}");
}
