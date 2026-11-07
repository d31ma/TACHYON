use super::*;

#[test]
fn authored_handlers_reject_helpers_in_every_language() {
    for (extension, source) in [
        ("js", "export class Handler {} class Delegate {}"),
        ("ts", "export class Handler {} interface Delegate {}"),
        ("py", "class Handler: pass\ndef helper(): pass\n"),
        ("php", "<?php class Handler {} function helper() {}"),
        ("java", "class Handler {} record Delegate(int value) {}"),
        ("kt", "class Handler {} fun helper() {}"),
        ("cs", "class Handler {} struct Delegate {}"),
        ("rs", "struct Handler; fn helper() {}"),
        ("swift", "@Checked\nstruct Handler {}\nfunc helper() {}"),
        ("dart", "class Handler {} void helper() {}"),
        ("go", "type Handler struct{}\nfunc helper() {}"),
    ] {
        let path = format!("server/workers/orders.{extension}");
        let error = validate(Path::new(&path), source).expect_err(extension);
        assert!(error.to_string().contains("TY1310"), "{error}");
    }
}

#[test]
fn annotated_files_refuse_every_extra_top_level_kind() {
    for extra in [
        "class Delegate {}",
        "interface Data {}",
        "type Data = string;",
        "enum Data { One }",
        "function helper() {}",
        "const value = 1;",
        "let value = 1;",
        "run();",
        "'use strict';",
        "export { OrdersService };",
    ] {
        let source = format!("@Service()\nexport class OrdersService {{}}\n{extra}");
        let error =
            validate(Path::new("server/services/OrdersService.ts"), &source).expect_err(extra);
        assert!(error.to_string().contains("one class"), "{extra}: {error}");
    }
}

#[test]
fn modifiers_and_test_annotations_activate_the_rule() {
    for marker in [
        "Checked",
        "Settings",
        "CacheSettings",
        "Lifetime",
        "NeedsAll",
        "Test",
        "TestSetup",
    ] {
        let source = format!("@{marker}({{}})\nclass Example {{}}\nclass Extra {{}}");
        validate(Path::new("client/tests/Example.ts"), &source).expect_err(marker);
    }
}

#[test]
fn plain_workers_and_root_middleware_are_checked_but_external_programs_are_not() {
    let source = "class Handler {} function helper() {}";
    for path in [
        "server/workers/nightly.js",
        "middleware.js",
        "server/workers/handler.js",
    ] {
        validate(Path::new(path), source).expect_err(path);
    }
    for path in [
        "server/shared/program.js",
        "server/helpers/program.js",
        "server/shared/middleware.js",
    ] {
        validate(Path::new(path), source).expect(path);
    }
}

#[test]
fn root_middleware_declares_the_middleware_class() {
    for (path, source) in [
        ("middleware.js", "export class Handler { static GET() {} }"),
        ("middleware.ts", "export class Gate { static before() {} }"),
        ("middleware.py", "class Handler:\n    pass\n"),
        ("middleware.php", "<?php\nfinal class Handler {}"),
        ("middleware.rs", "struct Handler;\nimpl Handler {}"),
    ] {
        let error = validate(Path::new(path), source).expect_err(path);
        let text = error.to_string();
        assert!(text.contains("TY1310"), "{path}: {text}");
        assert!(
            text.contains(
                "root middleware declares class Middleware with before(request) and after(request)"
            ),
            "{path}: {text}"
        );
    }
    for (path, source) in [
        (
            "middleware.js",
            "export class Middleware { static before() {} }",
        ),
        ("middleware.py", "class Middleware:\n    pass\n"),
        ("middleware.rs", "struct Middleware;\nimpl Middleware {}"),
    ] {
        validate(Path::new(path), source).expect(path);
    }
}

#[test]
fn one_class_imports_and_nested_types_work_in_every_language() {
    for (extension, source) in [
        (
            "js",
            "import {\n thing\n} from './other.js';\nexport class Handler { nested() { class Nested {} } }",
        ),
        (
            "ts",
            "import type {\n Other\n} from './other.ts'\nexport class Handler { value = 'class Fake {}'; }",
        ),
        (
            "py",
            "from typing import (\n Any,\n)\nimport json\nclass Handler:\n    class Nested: pass\n    value = '''class Fake:\n    pass\n'''\n",
        ),
        (
            "php",
            "<?php\nuse Example\\{Other, More};\nclass Handler { public const VALUE = 'class Fake {}'; }",
        ),
        (
            "java",
            "package example;\nimport java.util.Map;\nclass Handler { class Nested {} }",
        ),
        (
            "kt",
            "import example.Other\nclass Handler { companion object { class Nested {} } }",
        ),
        (
            "cs",
            "using System;\nusing Name = Example.Other;\nclass Handler { class Nested {} }",
        ),
        (
            "rs",
            "use std::{sync::Arc, collections::HashMap};\nstruct Handler<T>(T);\nimpl<T> Handler<T> {}\nimpl<T> SomeTrait<T> for Handler<T> {}",
        ),
        (
            "swift",
            "import Foundation\n@Checked\nstruct Handler { struct Nested {} }",
        ),
        (
            "dart",
            "import 'dart:convert' as convert;\nclass Handler { static const value = 'class Fake {}'; }",
        ),
        (
            "go",
            "package main\n\nimport \"strings\"\n\ntype Handler struct {\n\tvalue string\n}\n\nfunc (Handler) GET(request *http.Request) *http.Response {\n\treturn nil\n}\n",
        ),
    ] {
        let path = format!("server/workers/orders.{extension}");
        validate(Path::new(&path), source).unwrap_or_else(|error| panic!("{extension}: {error}"));
    }
}

#[test]
fn multiline_markers_and_same_line_declarations_are_not_hidden() {
    let source = "import { format }\nfrom '/shared/scripts/format.js';\n@Service(\n {\"label\": \"orders\"}\n)\nexport class OrdersService {}";
    assert!(
        validate(Path::new("server/services/OrdersService.ts"), source).expect("multiline marker")
    );
    let invalid = format!("{source} class Extra {{}}");
    validate(Path::new("server/services/OrdersService.ts"), &invalid).expect_err("same line class");
    let source =
        "#[yon::service(\n {}\n)]\npub(crate) struct OrdersService;\nimpl OrdersService {}";
    validate(Path::new("server/services/OrdersService.rs"), source).expect("Rust marker");
}

#[test]
fn rust_imports_may_carry_platform_conditions() {
    for imports in [
        "#[cfg(unix)]\nuse std::os::unix::net::UnixStream;",
        "#[cfg(windows)]\npub use std::os::windows::io::AsRawHandle;",
        "#[cfg(target_os = \"linux\")]\n#[cfg_attr(test, allow(unused_imports))]\nuse std::os::linux::fs::MetadataExt;",
        "use std::sync::Arc;\n#[cfg(all(unix, not(target_os = \"macos\")))]\nuse std::os::unix::fs::PermissionsExt;",
    ] {
        let source =
            format!("{imports}\n#[Service]\nstruct ProcessService;\nimpl ProcessService {{}}");
        validate(Path::new("client/services/ProcessService.rs"), &source).expect(imports);
    }
    for source in [
        "#[cfg(unix)]\nconst LIMIT: usize = 1;\n#[Service]\nstruct ProcessService;",
        "#[cfg(unix)]\nfn helper() {}\n#[Service]\nstruct ProcessService;",
        "#[cfg(unix)]\n#[cfg(test)]\nstatic LIMIT: usize = 1;\n#[Service]\nstruct ProcessService;",
        "#[Service]\nstruct ProcessService;\n#[cfg(unix)]\nstruct Extra;",
    ] {
        validate(Path::new("client/services/ProcessService.rs"), source).expect_err(source);
    }
}

#[test]
fn rust_requires_a_declared_type_and_only_its_implementations() {
    for source in [
        "#[Service]\nimpl OrdersService {}",
        "#[Service]\nstruct OrdersService; impl Other {}",
        "#[Service]\nstruct OrdersService; impl<T> Trait<T> for Other<T> {}",
        "#[Service]\nstruct OrdersService; struct Extra;",
        "#[Service]\nstruct OrdersService; const VALUE: usize = 1;",
    ] {
        validate(Path::new("server/services/OrdersService.rs"), source).expect_err(source);
    }
    for declaration in [
        "struct OrdersService;",
        "struct OrdersService(u64);",
        "enum OrdersService { Value }",
        "trait OrdersService {}",
    ] {
        let source = format!("#[Service]\n{declaration}");
        validate(Path::new("server/services/OrdersService.rs"), &source).expect(declaration);
    }
    let source = "#[Service]\npub struct OrdersService<'a, T, const N: usize> where T: Clone { value: &'a T }\nimpl<'a, T, const N: usize> Trait<T> for OrdersService<'a, T, N> where T: Clone {}";
    validate(Path::new("server/services/OrdersService.rs"), source).expect("generic impl");
    let source = "#[Service]\nstruct OrdersService<F>(F);\nimpl<F: Fn() -> usize> Trait for OrdersService<F> {}";
    validate(Path::new("server/services/OrdersService.rs"), source).expect("function-bound impl");
    let source = "#[doc = include_str!(\"guide.txt\")]\n#[Service]\nstruct OrdersService;";
    validate(Path::new("server/services/OrdersService.rs"), source).expect("compiler attribute");
}

#[test]
fn language_specific_no_body_declarations_are_bounded() {
    for (path, source) in [
        (
            "server/services/OrdersService.kt",
            "@Service\nclass OrdersService",
        ),
        (
            "server/services/OrdersService.cs",
            "[Service]\npublic record OrdersService(string Name);",
        ),
        (
            "server/services/OrdersService.java",
            "@Service\ninterface OrdersService {}",
        ),
    ] {
        validate(Path::new(path), source).expect(path);
        let extra = format!("{source}\nclass Extra {{}}");
        validate(Path::new(path), &extra).expect_err(path);
    }
}

#[test]
fn comments_and_literals_do_not_hide_top_level_code() {
    for source in [
        "class Handler {} /* comment */ 'extra';",
        "class Handler {} `extra`;",
        "class Handler {} /regex/;",
        "import './other.js'; run(); class Handler {}",
        "import('./other.js'); class Handler {}",
        "class Handler {} /* unclosed",
        "class Handler { value = 'unclosed",
    ] {
        validate(Path::new("server/workers/handler.js"), source).expect_err(source);
    }
    validate(
        Path::new("server/workers/handler.js"),
        "class Handler { pattern = /[\"{}]/; text = `class Fake {}`; }",
    )
    .expect("literal contents");
}

#[test]
fn a_worker_or_middleware_without_its_class_is_refused() {
    for (path, source) in [
        ("server/workers/handler.js", ""),
        ("server/workers/handler.js", "// no worker"),
        ("server/workers/handler.php", "<?php // no worker"),
        ("server/workers/handler.js", "import './controllers.js';"),
        ("middleware.js", "// no middleware"),
        ("server/workers/handler.rs", "impl Handler {}"),
        (
            "server/workers/handler.go",
            "package main\nimport \"strings\"\n",
        ),
        ("server/workers/nightly.go", "package main\n"),
    ] {
        validate(Path::new(path), source).expect_err(path);
    }
}

#[test]
fn grouped_attributes_find_framework_markers_after_foreign_attributes() {
    for (path, source) in [
        (
            "server/services/OrdersService.cs",
            "[Serializable, Service]\nclass OrdersService {}",
        ),
        (
            "server/services/OrdersService.php",
            "<?php #[Unknown(), Service]\nclass OrdersService {}",
        ),
    ] {
        assert!(validate(Path::new(path), source).expect(path));
        validate(Path::new(path), &format!("{source} class Extra {{}}")).expect_err(path);
    }
    validate(
        Path::new("server/shared/foreign.ts"),
        "@Other\nclass Service {} class Extra {}",
    )
    .expect("foreign annotations do not activate framework layout");
}

#[test]
fn adjacent_literals_comments_and_unicode_remain_separate_tokens() {
    for source in [
        "class Handler {}/* comment */'extra'",
        "import 'module'/*\ncomment */'extra'; class Handler {}",
        "class Handler {} /regex/",
        "class Handler {} '秘密'",
        "class Handleré {}",
    ] {
        let error = validate(Path::new("server/workers/handler.js"), source).expect_err(source);
        assert!(!error.to_string().contains("秘密"));
    }
    for source in [
        "\u{feff}class Handler { value = '日本語'; }",
        "/* 日本語 */ class Handler { value = `مرحبا`; }",
        "class Handler { value = 'x'/*inside*/; }",
    ] {
        validate(Path::new("server/workers/handler.js"), source).expect(source);
    }
    validate(
        Path::new("server/workers/handler.dart"),
        "class Handler { static const value = r'\\'; }",
    )
    .expect("raw Dart string");
}

#[test]
fn layout_diagnostics_point_to_the_extra_declaration() {
    let source = "class Handler {}\n// 日本語\nclass Extra {}";
    let error = validate(Path::new("server/workers/handler.ts"), source).expect_err("extra type");
    let diagnostic = &error.diagnostics()[0];
    assert_eq!(
        diagnostic.spans[0].start,
        u64::try_from(source.find("class Extra").expect("extra span")).expect("bounded span")
    );
    assert!(diagnostic.message.contains("Extra"));
    let help = diagnostic.help.as_deref().expect("relocation guidance");
    assert!(help.contains("server/models/"));
    assert!(help.contains("server/shared/"));
}

#[test]
fn dense_sources_stop_at_a_bounded_token_limit() {
    let source = format!("class Handler {{ {} }}", ";".repeat(262_144));
    let error =
        validate(Path::new("server/workers/handler.js"), &source).expect_err("token budget");
    assert!(error.to_string().contains("token limit"));
}

#[test]
fn php_mapping_interfaces_are_one_owned_type() {
    let path = Path::new("server/converters/OrdersConverter.php");
    let source = "<?php\n#[Converter]\ninterface OrdersConverter {\n#[Convert({})]\npublic function map(array $input): array;\n}";
    assert!(validate(path, source).expect("PHP mapper interface"));
    validate(path, &format!("{source}\ninterface Other {{}}")).expect_err("second interface");
}

#[test]
fn swift_mapping_protocols_are_one_owned_type() {
    let path = Path::new("client/converters/OrdersConverter.swift");
    let source = "@Converter\nprotocol OrdersConverter {\n@Convert({})\nfunc map(_ input: [String: Any]) -> [String: Any]\n}";
    assert!(validate(path, source).expect("Swift mapper protocol"));
    validate(path, &format!("{source}\nprotocol Other {{}}")).expect_err("second protocol");
}

#[test]
fn composition_marker_detection_includes_members_but_not_literals() {
    for source in [
        "@Checked({})\nclass Input {}",
        "class Settings {\n@Setting('setting')\nport;\n}",
    ] {
        assert!(
            has_framework_annotation(Path::new("server/shared/Value.ts"), source)
                .expect("marker scan")
        );
    }
    for source in [
        "class Plain {}",
        "const text = '@Service';",
        "/* @Checked */",
        "@Other\nclass Service {}",
    ] {
        assert!(
            !has_framework_annotation(Path::new("server/shared/Value.ts"), source)
                .expect("marker scan")
        );
    }
}

#[test]
fn indexes_operators_and_labels_are_not_annotations() {
    for (path, source) in [
        (
            "server/shared/Buffers.cs",
            "class Buffers { byte[] Make(int size) => new byte[size]; object Get(object[] row, int id) => row[id]; }",
        ),
        (
            "server/shared/Buffers.py",
            "def scale(matrix, size):\n    return matrix @ size\n",
        ),
        (
            "server/shared/Buffers.kt",
            "class Value { inner class Holder { fun owner() = this@Value } }\nclass Service { fun parent() = super@Service.toString() }",
        ),
    ] {
        assert!(
            !has_framework_annotation(Path::new(path), source).expect(path),
            "{path}"
        );
    }
    for (path, source) in [
        (
            "server/shared/Buffers.cs",
            "[Serializable][Service]\nclass Buffers {}",
        ),
        (
            "server/shared/Buffers.py",
            "@service\nclass Buffers:\n    pass\n",
        ),
        ("server/shared/Buffers.kt", "@Service\nclass Buffers"),
    ] {
        assert!(
            has_framework_annotation(Path::new(path), source).expect(path),
            "{path}"
        );
    }
}

#[test]
fn common_harmless_top_level_forms_are_accepted() {
    for (extension, source) in [
        (
            "py",
            "\"\"\"Money service.\"\"\"\nfrom typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    from models import Money\n@Service\nclass MoneyService:\n    total = 1 + \\\n2\n",
        ),
        (
            "js",
            "#!/usr/bin/env node\n\"use strict\";\n@Service\nexport default class MoneyService {}",
        ),
        (
            "php",
            "<?php\ndeclare(strict_types=1);\nuse function Money\\total;\nuse const Money\\LIMIT;\n#[Service]\nclass MoneyService {}\n?>\n",
        ),
        (
            "swift",
            "import struct Foundation.Date\n@Service\nfinal class MoneyService { let text = #\"a \"}\" b\"# }",
        ),
        (
            "dart",
            "library;\n@Service()\nabstract base class MoneyService {}",
        ),
        (
            "rs",
            "#![allow(dead_code)]\npub use crate::models::Money;\n#[Service]\npub struct MoneyService;",
        ),
        (
            "ts",
            "@Service\nexport default class MoneyService { text() { return `${ok ? `{` : ''}`; } }",
        ),
        (
            "cs",
            "[Service]\npublic class MoneyService { string Text => $\"{(ok ? \"}\" : \"\")}\"; }",
        ),
        (
            "kt",
            "@Service\nclass MoneyService\n@Needs\nconstructor(val money: Money) {}",
        ),
    ] {
        let path = format!("server/services/MoneyService.{extension}");
        validate(Path::new(&path), source).unwrap_or_else(|error| panic!("{extension}: {error}"));
    }
}

#[test]
fn accepted_forms_do_not_let_other_code_in() {
    for (extension, source) in [
        (
            "py",
            "if TYPE_CHECKING:\n    money = 1\n@Service\nclass MoneyService:\n    pass\n",
        ),
        (
            "py",
            "\"\"\"First.\"\"\"\n\"\"\"Second.\"\"\"\n@Service\nclass MoneyService:\n    pass\n",
        ),
        (
            "js",
            "@Service\nexport default class MoneyService {}\n\"use strict\";",
        ),
        (
            "php",
            "<?php\ndeclare(ticks=1) {}\n#[Service]\nclass MoneyService {}",
        ),
        ("php", "<?php\n#[Service]\nclass MoneyService {}\n?>\nhello"),
        (
            "dart",
            "@Service()\nclass MoneyService {}\nmixin Delegate {}",
        ),
        (
            "rs",
            "#[Service]\npub struct MoneyService;\npub fn helper() {}",
        ),
    ] {
        let path = format!("server/services/MoneyService.{extension}");
        let error = validate(Path::new(&path), source).expect_err(source);
        assert!(error.to_string().contains("one class"), "{error}");
    }
    validate(
        Path::new("server/workers/handler.py"),
        "\"\"\"Docstring.\"\"\"\n",
    )
    .expect_err("a docstring is not a comment-only route marker");
}

#[test]
fn diagnostics_name_a_dangling_annotation_and_a_second_annotated_class() {
    let path = Path::new("server/services/MoneyService.ts");
    let dangling = "@Service\nexport default class MoneyService {}\n@Other\n";
    let error = validate(path, dangling).expect_err("dangling annotation");
    assert_eq!(
        error.diagnostics()[0].message,
        "An annotated or handler file holds one class and its imports; found an annotation or modifier with no class after it"
    );
    let second = "@Service\nexport default class MoneyService {}\n@Service\nclass Other {}\n";
    let error = validate(path, second).expect_err("second annotated class");
    assert_eq!(
        error.diagnostics()[0].message,
        "An annotated or handler file holds one class and its imports; found another annotated class 'Other', which belongs in its own file"
    );
}

#[test]
fn namespace_package_and_nullable_lines_are_accepted() {
    for (path, source) in [
        (
            "server/services/MoneyService.kt",
            "package app.money\nimport app.models.Money\n@Service\nclass MoneyService",
        ),
        (
            "server/services/MoneyService.cs",
            "#nullable enable\nusing System;\nnamespace App.Money;\n[Service]\npublic class MoneyService {}",
        ),
        (
            "server/services/MoneyService.cs",
            "#nullable disable warnings\nnamespace App.Money {\nusing System;\n[Service]\npublic class MoneyService {}\n}",
        ),
        (
            "server/workers/orders.java",
            "package app.orders;\nclass Handler {}\n",
        ),
        (
            "server/workers/orders.java",
            "package app.orders;\nimport java.util.Map;\nclass Handler {}\n",
        ),
    ] {
        validate(Path::new(path), source).unwrap_or_else(|error| panic!("{source}: {error}"));
    }
}

#[test]
fn namespaces_do_not_let_other_code_in() {
    for (path, source) in [
        (
            "server/services/MoneyService.cs",
            "namespace App {\n[Service]\npublic class MoneyService {}\npublic class Other {}\n}",
        ),
        (
            "server/services/MoneyService.cs",
            "namespace App {\n[Service]\npublic class MoneyService {}\n}\nConsole.WriteLine(1);",
        ),
        (
            "server/services/MoneyService.cs",
            "#if DEBUG\n[Service]\npublic class MoneyService {}\n#endif",
        ),
        (
            "server/services/MoneyService.kt",
            "package app\n@Service\nclass MoneyService\nfun helper() = 1",
        ),
        ("server/workers/orders.kt", "package app\n"),
        (
            "server/workers/orders.java",
            "package app;\nstatic int value = 1;\n",
        ),
    ] {
        let error = validate(Path::new(path), source).expect_err(source);
        assert!(error.to_string().contains("one class"), "{error}");
    }
}

#[test]
fn literal_annotation_arguments_and_named_base_classes_are_accepted() {
    for (extension, source) in [
        (
            "ts",
            "@Needs(Counter, app.Clock)\n@Cached({\"cacheNames\":[\"products\"],\"ttl\":-1,\"sync\":true,\"key\":null})\n@Service\nexport default class MoneyService<T extends (value: string) => void> extends app.Base<T> implements Money {\n  static limit = 5;\n  @OnEvent({event: 'paid', replay: true}) paid() {}\n  @Setting(\"${yon.port:8080}\") port;\n}",
        ),
        (
            "py",
            "@dataclasses.dataclass(frozen=True)\n@Service\nclass MoneyService(Base, Generic[T], metaclass=Meta):\n    @cached({\"key_args\": [0], \"ttl\": None})\n    def total(self):\n        pass\n",
        ),
    ] {
        let path = format!("server/services/MoneyService.{extension}");
        validate(Path::new(&path), source).unwrap_or_else(|error| panic!("{source}: {error}"));
    }
}

#[test]
fn code_that_runs_when_the_file_loads_is_rejected() {
    for (extension, source, found) in [
        (
            "ts",
            "@Service\nexport default class MoneyService extends mixin(Base) {}",
            "code in the class header that runs when the file loads",
        ),
        (
            "ts",
            "@Service\nexport default class MoneyService extends (globalThis.x = 1, Object) {}",
            "code in the class header that runs when the file loads",
        ),
        (
            "py",
            "@Service\nclass MoneyService(make_base()):\n    pass\n",
            "code in the class header that runs when the file loads",
        ),
        (
            "ts",
            "@Service((globalThis.x = 1))\nexport default class MoneyService {}",
            "an annotation argument that runs code when the file loads",
        ),
        (
            "js",
            "@Service(`${load()}`)\nexport default class MoneyService {}",
            "an annotation argument that runs code when the file loads",
        ),
        (
            "ts",
            "@Service\nexport default class MoneyService { @Cached(options()) total() {} }",
            "an annotation argument that runs code when the file loads",
        ),
        (
            "py",
            "@Service\nclass MoneyService:\n    @cached(options())\n    def total(self):\n        pass\n",
            "an annotation argument that runs code when the file loads",
        ),
        (
            "ts",
            "@Service\nexport default class MoneyService { static { globalThis.hack = 1; } }",
            "a static block, which runs code when the file loads",
        ),
    ] {
        let path = format!("server/services/MoneyService.{extension}");
        let error = validate(Path::new(&path), source).expect_err(source);
        assert!(error.diagnostics()[0].message.contains(found), "{error}");
    }
}

struct GoSources;

impl GoSources {
    const HANDLER: &str = "package main\n\nimport \"fmt\"\nimport (\n\t\"strings\"\n\ttext \"text/template\"\n\t_ \"embed\"\n\t. \"math\"\n)\n\ntype Handler struct {\n\tcount int\n}\n\nfunc (Handler) GET(request *http.Request) *http.Response {\n\treturn nil\n}\n\nfunc (handler *Handler) POST(\n\trequest *http.Request,\n) any {\n\treturn map[string]any{\"value\": fmt.Sprint(Pi), \"template\": text.HTMLEscapeString(\"<\")}\n}\n\nfunc (h Handler) join(values ...string) (interface{}, error) {\n\treturn strings.Join(values, \",\"), nil\n}\n";
}

fn go_route_error(source: &str) -> String {
    let path = Path::new("server/workers/orders.go");
    validate(path, source).expect_err(source).to_string()
}

#[test]
fn a_go_file_holds_package_main_imports_and_one_type_with_its_methods() {
    let path = Path::new("server/workers/orders.go");
    validate(path, GoSources::HANDLER).expect("Go handler");
    let without_package = GoSources::HANDLER.replacen("package main", "", 1);
    validate(path, &without_package).expect("Go handler without a package clause");
    let middleware = "type Middleware struct{}\n\nfunc (Middleware) Before(request *http.Request) *http.Response {\n\treturn nil\n}\n";
    validate(Path::new("middleware.go"), middleware).expect("Go middleware");
    let worker =
        "type Handler int\n\nfunc (Handler) POST(request *http.Request) any { return nil }\n";
    validate(Path::new("server/workers/nightly.go"), worker).expect("Go worker");
}

#[test]
fn a_go_file_refuses_every_other_top_level_form() {
    for (extra, found) in [
        ("type Delegate struct{}", "another struct 'Delegate'"),
        ("type Names []string", "another type 'Names'"),
        ("type Alias = Handler", "a type alias 'Alias'"),
        ("type (\n\tA int\n)", "a grouped type declaration"),
        ("func helper() {}", "top-level 'func helper'"),
        ("func init() {}", "top-level 'func init'"),
        ("var count = 1", "top-level 'var'"),
        ("const limit = 2", "top-level 'const'"),
        ("func (Other) GET() {}", "a method of another type 'Other'"),
    ] {
        let source = format!("package main\n\ntype Handler struct{{}}\n\n{extra}\n");
        let error = go_route_error(&source);
        assert!(error.contains("TY1310"), "{extra}: {error}");
        assert!(error.contains(found), "{extra}: {error}");
    }
}

#[test]
fn a_go_file_names_its_package_main_and_its_entry_type() {
    let error = go_route_error("package handlers\n\ntype Handler struct{}\n");
    assert!(error.contains("compiled as package main"), "{error}");
    let error = go_route_error("type Orders struct{}\n");
    assert!(
        error.contains("'Orders' instead of the 'Handler' entry class"),
        "{error}"
    );
    let error = validate(Path::new("middleware.go"), "type Handler struct{}\n")
        .expect_err("a Go middleware with a Handler type")
        .to_string();
    assert!(
        error.contains(
            "Go root middleware declares type Middleware with Before(request) and After(request)"
        ),
        "{error}"
    );
}

#[test]
fn go_annotations_sit_on_the_one_type_and_its_methods() {
    for (path, source) in [
        (
            "server/services/MoneyService.go",
            "// @Service\ntype MoneyService struct {\n\t// @Needs\n\tstore *MoneyStore\n}\n",
        ),
        (
            "server/services/MoneyService.go",
            "import \"strings\"\n\n// @yon::Service\n// @CacheSettings({\"name\":\"money\",\"ttlMs\":1000,\"maxBytes\":4096,\"maxEntries\":10})\ntype MoneyService struct{}\n\n// @Cached({\"keyArgs\":[0]})\nfunc (service *MoneyService) Total(id BehaviorValue) (BehaviorValue, error) {\n\treturn strings.ToUpper(\"x\"), nil\n}\n",
        ),
        (
            "server/workers/orders.go",
            "// @Schedule({\"fixedDelaySeconds\":60})\ntype Handler struct{}\n\nfunc (Handler) POST(request *http.Request) any { return nil }\n",
        ),
    ] {
        assert!(
            validate(Path::new(path), source).is_ok(),
            "{path}: {source}"
        );
    }
    let misplaced = validate(
        Path::new("server/helpers/MoneyService.go"),
        "// @Service\ntype MoneyService struct{}\n",
    )
    .expect_err("a service outside its role folder")
    .to_string();
    assert!(misplaced.contains("TY1310"), "{misplaced}");
    let other = validate(
        Path::new("server/services/MoneyService.go"),
        "// @Service\ntype MoneyService struct{}\n\n// @Cached({})\nfunc (Other) Total() {}\n",
    )
    .expect_err("a method of another type")
    .to_string();
    assert!(
        other.contains("a method of another type 'Other'"),
        "{other}"
    );
}

#[test]
fn python_markers_inside_annotated_types_make_a_framework_file() {
    let path = Path::new("server/helpers/ValidationService.py");
    let checked = "from typing import Annotated\n\nclass ValidationService:\n    def accept(self, value: Annotated[ProductInput, check]):\n        return value\n";
    assert!(has_framework_annotation(path, checked).expect("checked"));
    let bounded =
        "from typing import Annotated\n\nclass Settings:\n    size: Annotated[int, min(1)] = 1\n";
    assert!(has_framework_annotation(path, bounded).expect("bounded"));
    let plain =
        "from typing import Annotated\n\nclass Plain:\n    size: Annotated[int, \"doc\"] = 1\n";
    assert!(!has_framework_annotation(path, plain).expect("plain"));
}
