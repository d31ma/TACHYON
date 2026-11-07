use super::{EvalCheck, QueryCheck};
use std::path::Path;

fn evals(path: &str, source: &str) -> usize {
    EvalCheck::diagnostics([(Path::new(path), source.as_bytes())].into_iter()).len()
}

fn queries(path: &str, source: &str) -> Vec<String> {
    QueryCheck::diagnostics([(Path::new(path), source.as_bytes())].into_iter())
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

#[test]
fn eval_style_calls_are_refused_in_every_language() {
    for (path, source) in [
        ("server/services/A.js", "run(code) { return eval(code) }\n"),
        (
            "client/services/A.ts",
            "const run = new Function('a', body);\n",
        ),
        (
            "server/services/A.py",
            "def run(self, code):\n    exec(code)\n",
        ),
        (
            "server/services/A.py",
            "def run(self, code):\n    return eval(code)\n",
        ),
        (
            "server/services/A.rb",
            "def run(code)\n  instance_eval(code)\nend\n",
        ),
        ("server/services/A.rb", "def run(code)\n  eval code\nend\n"),
        (
            "server/services/A.php",
            "<?php\nfunction run($code) { return eval($code); }\n",
        ),
        (
            "server/services/A.java",
            "Object run(String code) throws Exception { return new javax.script.ScriptEngineManager().getEngineByName(\"js\").eval(code); }\n",
        ),
        (
            "server/services/A.cs",
            "object Run(string code) => CSharpScript.EvaluateAsync(code).Result;\n",
        ),
    ] {
        assert!(evals(path, source) >= 1, "{path}: {source}");
    }
}

#[test]
fn words_that_only_look_like_eval_are_allowed() {
    for (path, source) in [
        (
            "server/services/A.js",
            "// eval(code) is refused\nconst text = 'eval(x)';\nthis.evaluate(x);\nmodel.eval(x);\n",
        ),
        (
            "server/services/A.py",
            "retrieval(x)\nself.executor.exec(x)\n",
        ),
        (
            "server/services/A.rb",
            "def run\n  instance_eval { value }\n  class_eval do\n  end\nend\n",
        ),
        ("server/services/A.go", "func run() { eval(x) }\n"),
    ] {
        assert_eq!(evals(path, source), 0, "{path}: {source}");
    }
}

#[test]
fn query_text_built_from_pieces_is_refused() {
    for (path, source) in [
        (
            "server/stores/A.js",
            "find(name) { return this.dataSource.query('SELECT * FROM t WHERE n = ' + name, []) }\n",
        ),
        (
            "server/stores/A.ts",
            "find(name: string) { return this.dataSource.query(`SELECT * FROM t WHERE n = ${name}`, []) }\n",
        ),
        (
            "server/stores/A.py",
            "def find(self, name):\n    return self.data_source.query(f\"SELECT * FROM t WHERE n = '{name}'\", [])\n",
        ),
        (
            "server/stores/A.py",
            "def find(self, name):\n    return self.cursor.execute(\"SELECT * FROM t WHERE n = '%s'\" % name)\n",
        ),
        (
            "server/stores/A.rb",
            "def find(name)\n  data_source.query(\"SELECT * FROM t WHERE n = '#{name}'\", [])\nend\n",
        ),
        (
            "server/stores/A.php",
            "<?php\nfunction find($name) { return $this->dataSource->query(\"SELECT * FROM t WHERE n = '$name'\", []); }\n",
        ),
        (
            "server/stores/A.java",
            "private final DataSource store;\nObject find(String name) { return store.query(\"SELECT * FROM t WHERE n = '\" + name + \"'\", java.util.List.of()); }\n",
        ),
        (
            "server/stores/A.cs",
            "object Find(string name) => dataSource.Query($\"SELECT * FROM t WHERE n = '{name}'\", new object[0]);\n",
        ),
        (
            "server/stores/A.go",
            "func (store *A) Find(name string) { store.db.Query(fmt.Sprintf(\"SELECT * FROM t WHERE n = '%s'\", name)) }\n",
        ),
        (
            "server/stores/A.kt",
            "fun find(name: String) = dataSource.query(\"SELECT * FROM t WHERE n = '$name'\", listOf())\n",
        ),
        (
            "server/stores/A.js",
            "find(sql) { return this.dataSource.query(sql, []) }\n",
        ),
    ] {
        let found = queries(path, source);
        assert_eq!(found.len(), 1, "{path}: {source}");
        assert!(
            found[0].contains("builds query text from pieces"),
            "{found:?}"
        );
    }
}

#[test]
fn parameterized_queries_and_constants_are_allowed() {
    for (path, source) in [
        (
            "server/stores/A.js",
            "find(name) {\n  return this.dataSource.query(\n    'SELECT id, name FROM products WHERE name = $1', [name]\n  )\n}\n",
        ),
        (
            "server/stores/A.ts",
            "static FIND = 'SELECT 1';\nfind() { return this.dataSource.query(A.FIND, []) }\n",
        ),
        (
            "server/stores/A.py",
            "def find(self, name):\n    return self.data_source.query(\"SELECT * FROM t WHERE n = $1\", [name])\n",
        ),
        (
            "server/stores/A.php",
            "<?php\nfunction find($name) { return $this->dataSource->query('SELECT * FROM t WHERE n = $1', [$name]); }\n",
        ),
        (
            "server/stores/A.cs",
            "object Find(string name) => dataSource.Query(@\"SELECT * FROM t WHERE n = $1\", new object[] { name });\n",
        ),
        (
            "server/stores/A.go",
            "func (store *A) Find(name string) { store.db.Query(\"SELECT * FROM t WHERE n = $1\", name) }\n",
        ),
        (
            "server/controllers/A.go",
            "func (c *A) Get(request *http.Request) { request.URL.Query() }\n",
        ),
        (
            "server/stores/A.js",
            "run() { return this.executor.execute(task) }\n",
        ),
        (
            "server/stores/A.js",
            "read(text) { return /a+/.exec(text) ?? this.feedback.execute(text) }\n",
        ),
    ] {
        assert!(queries(path, source).is_empty(), "{path}: {source}");
    }
}
