use super::{
    CacheDirectory, Compiler, CompilerOutput, EntryClass, Failure, PreparedCommand,
    bounded_command_output, build_once_under_lock, check_compiler_output, compiled_digest,
    compiler_command, compiler_workspace, digest_source, invalid_source, missing_compiler,
    names_a_missing_program, prepare_cached_artifact, valid_owned_artifact, write_compiler_file,
    write_model_files,
};
use crate::failure::{diagnostic, source_span};
use crate::go_source::{GoFile, GoMethod};
use crate::yon::go_container::GoRouteResult;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(super) struct GoHandler;

pub(super) struct GoSharedCode;

struct GoFiles<'a> {
    handler: &'a str,
    runtime: &'a str,
    models: &'a [(PathBuf, String)],
}

impl GoHandler {
    const MODULE_FILE: &str = "module yonhandler\n\ngo 1.27\n";
    const PRELUDE: &str = include_str!("../preludes/yon.go");
    const INVOCATION_PACKAGE: &str = include_str!("../preludes/yon_invocation.go");
    const ENVIRONMENT: [(&str, &str); 6] = [
        ("GO111MODULE", "on"),
        ("GOFLAGS", ""),
        ("GOPROXY", "off"),
        ("GOTOOLCHAIN", "local"),
        ("GOWORK", "off"),
        ("CGO_ENABLED", "0"),
    ];
    const OTHER_TARGET_NAMES: [&str; 2] = ["GOOS", "GOARCH"];
    const COMPILER: Compiler = Compiler {
        compile: &["go"],
        run: &[],
        suffix: if cfg!(windows) { ".exe" } else { "" },
        lists_model_files: false,
    };
    const MIDDLEWARE_METHODS: [(&str, &str); 2] = [
        ("middleware.before", "Before"),
        ("middleware.after", "After"),
    ];
    const ROUTE_SHAPE_HELP: &str = "A Go route method takes no parameter or one request *http.Request, and returns nothing, a value, an error, or a value and an error.";
    const NATIVE_REQUEST: &str = "*http.Request";

    pub(super) fn is_go(source: &Path) -> bool {
        source
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("go"))
    }

    pub(super) fn compile(
        bytes: &[u8],
        portable: &str,
        cache: &CacheDirectory,
        model_files: &[(PathBuf, String)],
    ) -> Result<PreparedCommand, Failure> {
        let authored = String::from_utf8_lossy(bytes).into_owned();
        let mut parts = GoFile::split(&authored);
        let entry = parts.remove(0);
        let handler_file = Self::with_package_clause(&entry.text, portable)?;
        let mut staged_files = model_files.to_vec();
        staged_files.extend(Self::program_files(parts, portable)?);
        let model_files = staged_files.as_slice();
        let runtime_file = Self::runtime_file(&authored, EntryClass::of(portable), portable)?;
        let runtime_digest = format!("{runtime_file}{}", Self::INVOCATION_PACKAGE);
        let digest = compiled_digest(
            digest_source(&handler_file, model_files).as_bytes(),
            runtime_digest.as_bytes(),
            &Self::compiler_identity(portable)?,
            std::env::consts::ARCH,
            std::env::consts::OS,
        );
        let artefact = PathBuf::from(format!("{digest}{}", Self::COMPILER.suffix));
        if !cache.is_valid_file(&artefact) {
            build_once_under_lock(cache, &artefact, &digest, portable, || {
                let files = GoFiles {
                    handler: &handler_file,
                    runtime: &runtime_file,
                    models: model_files,
                };
                Self::build(&files, &artefact, portable, cache)
            })?;
        }
        prepare_cached_artifact(&Self::COMPILER, &artefact, &digest, portable, cache)
    }

    fn with_package_clause(authored: &str, portable: &str) -> Result<String, Failure> {
        let code = crate::annotation_layout::masked_source(Path::new(portable), authored)?;
        if code.split_whitespace().next() == Some("package") {
            return Ok(authored.to_owned());
        }
        Ok(format!("package main; {}", crate::without_bom(authored)))
    }

    fn program_files(
        parts: Vec<GoFile>,
        portable: &str,
    ) -> Result<Vec<(PathBuf, String)>, Failure> {
        let mut used = BTreeSet::from([String::from("handler"), String::from("yon")]);
        let mut files = Vec::new();
        for (index, part) in parts.into_iter().enumerate() {
            let mut name = Self::file_name(&part.name);
            if used.contains(&name) || name.contains('_') {
                name = format!("{name}{index}");
            }
            used.insert(name.clone());
            let text = Self::with_package_clause(&part.text, portable)?;
            files.push((PathBuf::from(format!("{name}.go")), text));
        }
        Ok(files)
    }

    fn file_name(name: &str) -> String {
        let mut file = String::new();
        for character in name.chars() {
            if character.is_ascii_alphanumeric() {
                file.push(character);
            } else {
                file.push('_');
            }
        }
        if file.is_empty() || file.starts_with('_') {
            file.insert_str(0, "part");
        }
        file
    }

    fn runtime_file(authored: &str, entry: EntryClass, portable: &str) -> Result<String, Failure> {
        let code = crate::annotation_layout::masked_source(Path::new(portable), authored)?;
        let declared = Self::declared_methods(&code, entry);
        if matches!(entry, EntryClass::Middleware) {
            Self::check_middleware_method_names(&declared, portable)?;
        }
        let mut runtime = String::from(Self::PRELUDE);
        runtime.push_str(&Self::dispatch(entry, &code, &declared, portable)?);
        Ok(runtime)
    }

    fn check_middleware_method_names(
        declared: &BTreeMap<String, GoMethod>,
        portable: &str,
    ) -> Result<(), Failure> {
        for (_, method) in Self::MIDDLEWARE_METHODS {
            let lower = method.to_ascii_lowercase();
            if declared.contains_key(&lower) {
                return Err(Self::failure(
                    2015,
                    portable,
                    format!(
                        "Go middleware names its phases with exported methods: rename '{lower}' to '{method}'."
                    ),
                    "Tachyon calls Before(request) before the route and After(request) after it. A Go method whose name starts with a lower-case letter is never called.",
                ));
            }
        }
        Ok(())
    }

    fn failure(code: u16, portable: &str, message: String, help: &str) -> Failure {
        Failure::one(diagnostic(
            code,
            message,
            Some(String::from(help)),
            source_span(portable, 0, portable.len()),
        ))
    }

    fn dispatch(
        entry: EntryClass,
        code: &str,
        declared: &BTreeMap<String, GoMethod>,
        portable: &str,
    ) -> Result<String, Failure> {
        let field = match entry {
            EntryClass::Handler => "Method",
            EntryClass::Middleware => "Operation",
        };
        let mut text = String::from("\nfunc yonDispatch(request *yonRequest) *yonResponse {\n");
        if matches!(entry, EntryClass::Handler) && declared.contains_key("YonDeliver") {
            text.push_str("\tif request.fields.Operation == \"event.deliver\" && request.fields.Body != nil {\n\t\tHandler{}.YonDeliver(request.fields.Body.Data)\n\t\treturn yonEmpty(204)\n\t}\n");
        }
        let _ = writeln!(text, "\tswitch request.fields.{field} {{");
        for (operation, method) in Self::operations(entry) {
            if let Some(found) = declared.get(method) {
                let answer = Self::answer(entry, code, found, portable)?;
                let _ = writeln!(text, "\tcase \"{operation}\":\n{answer}");
            } else if matches!(entry, EntryClass::Middleware) {
                let _ = write!(text, "\tcase \"{operation}\":\n\t\treturn yonEmpty(204)\n");
            }
        }
        text.push_str("\t}\n\treturn yonEmpty(405)\n}\n");
        Ok(text)
    }

    fn answer(
        entry: EntryClass,
        code: &str,
        method: &GoMethod,
        portable: &str,
    ) -> Result<String, Failure> {
        let name = &method.name;
        let parameters = crate::go_source::parameters(&code[method.parameters.clone()]);
        let argument = match parameters.first() {
            None => "",
            Some(parameter) if parameter.type_source == Self::NATIVE_REQUEST => {
                "yonNativeRequest(request)"
            }
            Some(_) => "request",
        };
        if matches!(entry, EntryClass::Middleware) {
            return Ok(format!(
                "\t\treturn yonAnswer(new(Middleware).{name}({argument}))"
            ));
        }
        if parameters.len() > 1 {
            return Err(Self::failure(
                2015,
                portable,
                format!("Go route method Handler.{name} takes more than one parameter."),
                Self::ROUTE_SHAPE_HELP,
            ));
        }
        let call = format!("new(Handler).{name}({argument})");
        let result = GoRouteResult::of(&code[method.results.clone()]).map_err(|message| {
            Self::failure(
                2015,
                portable,
                format!("{message}; Handler.{name} does not."),
                Self::ROUTE_SHAPE_HELP,
            )
        })?;
        let answer = match result {
            GoRouteResult::Nothing => format!("\t\t{call}\n\t\treturn yonAnswer(nil)"),
            GoRouteResult::Value | GoRouteResult::ValueAndFailure => {
                format!("\t\treturn yonAnswer({call})")
            }
            GoRouteResult::Failure => format!("\t\treturn yonAnswer(nil, {call})"),
        };
        Ok(answer)
    }

    fn operations(entry: EntryClass) -> Vec<(&'static str, &'static str)> {
        match entry {
            EntryClass::Handler => EntryClass::HTTP_METHODS
                .iter()
                .map(|method| (*method, *method))
                .collect(),
            EntryClass::Middleware => Self::MIDDLEWARE_METHODS.to_vec(),
        }
    }

    fn declared_methods(code: &str, entry: EntryClass) -> BTreeMap<String, GoMethod> {
        let type_name = match entry {
            EntryClass::Handler => "Handler",
            EntryClass::Middleware => "Middleware",
        };
        let mut methods = BTreeMap::new();
        for method in crate::go_source::methods(code) {
            if method.receiver_type == type_name {
                methods.insert(method.name.clone(), method);
            }
        }
        methods
    }

    fn command(arguments: &[String], directory: Option<&Path>) -> std::process::Command {
        let mut command = compiler_command("go", arguments, directory);
        command.envs(Self::ENVIRONMENT);
        for name in Self::OTHER_TARGET_NAMES {
            command.env_remove(name);
        }
        command
    }

    fn write_invocation_package(directory: &Path, portable: &str) -> Result<(), Failure> {
        let package = directory.join("yoninvocation");
        std::fs::create_dir(&package).map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot create the Go invocation package: {error}"),
            )
        })?;
        write_compiler_file(
            &package.join("invocation.go"),
            Self::INVOCATION_PACKAGE.as_bytes(),
            portable,
        )
    }

    fn compiler_identity(portable: &str) -> Result<String, Failure> {
        let command = Self::command(&[String::from("version")], None);
        let result =
            bounded_command_output(command, CompilerOutput::DEADLINE, CompilerOutput::MAX_BYTES);
        match result {
            Ok(output) => Ok(format!(
                "go:{}",
                String::from_utf8_lossy(&output.stdout).trim()
            )),
            Err(error) if names_a_missing_program(&error) => Err(missing_compiler("go", portable)),
            Err(_) => Ok(String::from("go")),
        }
    }

    fn build(
        files: &GoFiles<'_>,
        artefact: &Path,
        portable: &str,
        cache: &CacheDirectory,
    ) -> Result<(), Failure> {
        let workspace = compiler_workspace(
            "tachyon-go-build-",
            "Cannot create Go compiler workspace",
            portable,
        )?;
        let directory = workspace.path();
        write_compiler_file(
            &directory.join("go.mod"),
            Self::MODULE_FILE.as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &directory.join("handler.go"),
            files.handler.as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &directory.join("yon.go"),
            files.runtime.as_bytes(),
            portable,
        )?;
        write_model_files(directory, files.models, portable)?;
        Self::write_invocation_package(directory, portable)?;
        let output = directory.join(format!("output{}", Self::COMPILER.suffix));
        let arguments = [
            String::from("build"),
            String::from("-trimpath"),
            String::from("-buildvcs=false"),
            String::from("-o"),
            output.to_string_lossy().into_owned(),
            String::from("."),
        ];
        let command = Self::command(&arguments, Some(directory));
        let result =
            bounded_command_output(command, CompilerOutput::DEADLINE, CompilerOutput::MAX_BYTES);
        check_compiler_output("go", result, portable)?;
        if !valid_owned_artifact(&output) {
            return Err(invalid_source(
                portable,
                "Compiler completed without producing a valid handler artifact.",
            ));
        }
        cache.publish_file(&output, artefact).map_err(|error| {
            super::cache_failure(portable, "Cannot publish the handler artifact", error)
        })
    }
}

impl GoSharedCode {
    const FOLDER: &str = "server/shared";
    const HELP: &str = "server/shared/ holds plain Go code of package main. Put an annotated type in its role folder, for example server/services/.";

    pub(super) fn read(source_root: &Path) -> Result<Vec<(PathBuf, String)>, Failure> {
        let mut paths = Vec::new();
        Self::find_go_files(source_root, Path::new(Self::FOLDER), &mut paths);
        paths.sort();
        let mut files = Vec::new();
        for (index, relative) in paths.iter().enumerate() {
            let text = Self::staged_text(source_root, relative)?;
            files.push((PathBuf::from(format!("shared_{index}.go")), text));
        }
        Ok(files)
    }

    fn find_go_files(source_root: &Path, relative: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(source_root.join(relative)) else {
            return;
        };
        for entry in entries.flatten() {
            let child = relative.join(entry.file_name());
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                Self::find_go_files(source_root, &child, found);
            } else if kind.is_file() && GoHandler::is_go(&child) {
                found.push(child);
            }
        }
    }

    fn staged_text(source_root: &Path, relative: &Path) -> Result<String, Failure> {
        let portable = relative.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(source_root.join(relative)).map_err(|error| {
            Self::failure(&portable, format!("Cannot read '{portable}': {error}"))
        })?;
        let Ok(source) = String::from_utf8(bytes) else {
            return Err(Self::failure(
                &portable,
                format!("'{portable}' must be UTF-8."),
            ));
        };
        let source = crate::without_bom(&source);
        if crate::annotation_layout::has_framework_annotation(relative, source)? {
            return Err(Self::failure(
                &portable,
                format!("'{portable}' is plain shared code and cannot carry a Tachyon annotation."),
            ));
        }
        let code = crate::annotation_layout::masked_source(relative, source)?;
        let mut words = code.split_whitespace();
        let position = format!("//line {portable}:1\n");
        if words.next() != Some("package") {
            return Ok(format!("{position}package main; {source}"));
        }
        if words.next() != Some("main") {
            return Err(Self::failure(
                &portable,
                format!("'{portable}' is compiled as package main; found another package clause."),
            ));
        }
        Ok(format!("{position}{source}"))
    }

    fn failure(portable: &str, message: String) -> Failure {
        Failure::one(diagnostic(
            1310,
            message,
            Some(String::from(Self::HELP)),
            source_span(portable, 0, 0),
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{EntryClass, GoHandler};

    fn runtime(source: &str, entry: EntryClass, portable: &str) -> String {
        GoHandler::runtime_file(source, entry, portable).expect("Go runtime")
    }

    #[test]
    fn a_route_dispatch_calls_only_the_verbs_the_handler_declares() {
        let source = "type Handler struct{}\n\nfunc (Handler) GET(request *http.Request) *http.Response { return nil }\n\nfunc (h *Handler) POST(\n\trequest *http.Request,\n) any { return `func (Handler) PUT(` }\n";
        let dispatch = runtime(source, EntryClass::Handler, "server/routes/yon.go");
        assert!(
            dispatch.contains(
                "case \"GET\":\n\t\treturn yonAnswer(new(Handler).GET(yonNativeRequest(request)))"
            ),
            "{dispatch}"
        );
        assert!(
            dispatch.contains(
                "case \"POST\":\n\t\treturn yonAnswer(new(Handler).POST(yonNativeRequest(request)))"
            ),
            "{dispatch}"
        );
        assert!(!dispatch.contains("case \"PUT\""), "{dispatch}");
        assert!(
            dispatch.ends_with("\t}\n\treturn yonEmpty(405)\n}\n"),
            "{dispatch}"
        );
    }

    #[test]
    fn a_route_dispatch_follows_each_parameter_and_result_shape() {
        let source = "type Handler struct{}\n\nfunc (Handler) GET() any { return 1 }\n\nfunc (Handler) PUT() error { return nil }\n\nfunc (Handler) PATCH() {}\n";
        let dispatch = runtime(source, EntryClass::Handler, "server/routes/yon.go");
        assert!(
            dispatch.contains("case \"GET\":\n\t\treturn yonAnswer(new(Handler).GET())"),
            "{dispatch}"
        );
        assert!(
            dispatch.contains("case \"PUT\":\n\t\treturn yonAnswer(nil, new(Handler).PUT())"),
            "{dispatch}"
        );
        assert!(
            dispatch
                .contains("case \"PATCH\":\n\t\tnew(Handler).PATCH()\n\t\treturn yonAnswer(nil)"),
            "{dispatch}"
        );
    }

    #[test]
    fn a_route_method_with_three_results_fails_with_ty2015() {
        let source = "type Handler struct{}\n\nfunc (Handler) GET() (int, int, error) { return 1, 2, nil }\n";
        let failure = GoHandler::runtime_file(source, EntryClass::Handler, "server/routes/yon.go")
            .expect_err("three results");
        let rendered = failure.to_string();
        assert!(rendered.contains("TY2015"), "{rendered}");
        assert!(rendered.contains("Handler.GET does not"), "{rendered}");
    }

    #[test]
    fn a_middleware_dispatch_passes_a_missing_phase() {
        let source = "type Middleware struct{}\n\nfunc (Middleware) After(request *http.Request) *http.Response { return nil }\n";
        let dispatch = runtime(source, EntryClass::Middleware, "middleware.go");
        assert!(
            dispatch.contains("switch request.fields.Operation"),
            "{dispatch}"
        );
        assert!(
            dispatch.contains("case \"middleware.before\":\n\t\treturn yonEmpty(204)"),
            "{dispatch}"
        );
        assert!(
            dispatch.contains(
                "case \"middleware.after\":\n\t\treturn yonAnswer(new(Middleware).After(yonNativeRequest(request)))"
            ),
            "{dispatch}"
        );
    }

    #[test]
    fn a_middleware_phase_is_an_exported_method() {
        let source = "type Middleware struct{}\n\nfunc (Middleware) before(request *http.Request) *http.Response { return nil }\n";
        let failure = GoHandler::runtime_file(source, EntryClass::Middleware, "middleware.go")
            .expect_err("a lower-case phase would never run");
        assert!(
            failure.to_string().contains("rename 'before' to 'Before'"),
            "{failure}"
        );
    }

    #[test]
    fn handler_code_shares_package_main_with_no_relay_state() {
        let mut variables = Vec::new();
        for line in GoHandler::PRELUDE.lines() {
            if line.starts_with("var ") {
                variables.push(line);
            }
        }
        assert_eq!(variables, ["var YonGateway = YonGatewayClient{}"]);
        assert!(GoHandler::INVOCATION_PACKAGE.starts_with("package yoninvocation\n"));
    }

    #[test]
    fn a_handler_file_without_a_package_clause_is_compiled_as_package_main() {
        let with_clause = "// orders\npackage main\n\ntype Handler struct{}\n";
        let kept =
            GoHandler::with_package_clause(with_clause, "server/routes/yon.go").expect("kept");
        assert_eq!(kept, with_clause);
        let without_clause = "// orders\ntype Handler struct{}\n";
        let added =
            GoHandler::with_package_clause(without_clause, "server/routes/yon.go").expect("added");
        assert_eq!(added, "package main; // orders\ntype Handler struct{}\n");
    }
}
