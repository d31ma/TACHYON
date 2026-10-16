use super::groovy_build::GroovyBuild;
use super::{
    CacheDirectory, Compiler, CompilerOutput, EntryClass, Failure, PreparedCommand,
    bounded_compiler_output, build_once_under_lock, compiler_workspace, invalid_source,
    missing_compiler, names_a_missing_program, prepare_cached_artifact, protocol_prelude,
    valid_owned_artifact, write_compiler_file,
};
use crate::groovy_home::GroovyHome;
use crate::groovy_source::GroovySource;
use std::path::{Path, PathBuf};

pub(super) struct GroovyHandler;

impl GroovyHandler {
    const COMPILER: Compiler = Compiler {
        compile: &["java"],
        run: &["java", "-jar"],
        suffix: ".jar",
        lists_model_files: false,
    };
    const LAUNCHER: &'static str = "groovyc";
    const JAVA: &'static str = "java";
    const JAR: &'static str = "jar";
    const GROOVY_FILE: &'static str = "Program.groovy";
    const JAVA_FILE: &'static str = "Yon.java";
    const CLASSES_FOLDER: &'static str = "classes";
    const OUTPUT_FILE: &'static str = "output.jar";
    const MAIN_CLASS: &'static str = "YonLauncher";
    const COMPILER_CLASS: &'static str = "org.codehaus.groovy.tools.FileSystemCompiler";
    const RUNTIME_MANIFEST: [&'static str; 2] = ["META-INF", "MANIFEST.MF"];
    const SOURCE_NAMES: [&'static str; 2] = [".groovy:", ".java:"];

    pub(super) fn compile(
        bytes: &[u8],
        portable: &str,
        cache: &CacheDirectory,
    ) -> Result<PreparedCommand, Failure> {
        let program = String::from_utf8_lossy(bytes).into_owned();
        let (groovy, generated) = GroovySource::split(&program);
        let prelude = protocol_prelude(Self::JAVA, groovy, EntryClass::of(portable));
        let java = format!("{prelude}\n{generated}");
        let home = GroovyHome::find().ok_or_else(|| missing_compiler(Self::LAUNCHER, portable))?;
        let runtime_jar = home
            .runtime_jar()
            .ok_or_else(|| missing_compiler(Self::LAUNCHER, portable))?;
        let build = GroovyBuild {
            groovy,
            java: &java,
            home: &home,
            runtime_jar: &runtime_jar,
        };
        let digest = build.digest();
        let artefact = PathBuf::from(format!("{digest}{}", Self::COMPILER.suffix));
        Self::build_when_missing(&build, &artefact, &digest, portable, cache)?;
        prepare_cached_artifact(&Self::COMPILER, &artefact, &digest, portable, cache)
    }

    fn build_when_missing(
        build: &GroovyBuild<'_>,
        artefact: &Path,
        digest: &str,
        portable: &str,
        cache: &CacheDirectory,
    ) -> Result<(), Failure> {
        if cache.is_valid_file(artefact) {
            return Ok(());
        }
        build_once_under_lock(cache, artefact, digest, portable, || {
            Self::build(build, artefact, portable, cache)
        })
    }

    fn build(
        build: &GroovyBuild<'_>,
        artefact: &Path,
        portable: &str,
        cache: &CacheDirectory,
    ) -> Result<(), Failure> {
        let workspace = compiler_workspace(
            "tachyon-groovy-build-",
            "Cannot create compiler workspace",
            portable,
        )?;
        let folder = workspace.path();
        let groovy_file = folder.join(Self::GROOVY_FILE);
        write_compiler_file(&groovy_file, build.groovy.as_bytes(), portable)?;
        let java_file = folder.join(Self::JAVA_FILE);
        write_compiler_file(&java_file, build.java.as_bytes(), portable)?;
        let classes = folder.join(Self::CLASSES_FOLDER);
        std::fs::create_dir(&classes).map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot create the compiled classes folder: {error}"),
            )
        })?;
        let compile_arguments = Self::compile_arguments(build.home);
        Self::run(Self::JAVA, &compile_arguments, folder, portable)?;
        Self::add_runtime(build.runtime_jar, &classes, portable)?;
        Self::run(Self::JAR, &Self::jar_arguments(), folder, portable)?;
        Self::publish(&folder.join(Self::OUTPUT_FILE), artefact, portable, cache)
    }

    fn publish(
        output: &Path,
        artefact: &Path,
        portable: &str,
        cache: &CacheDirectory,
    ) -> Result<(), Failure> {
        if !valid_owned_artifact(output) {
            return Err(invalid_source(
                portable,
                "Compiler completed without producing a valid handler artifact.",
            ));
        }
        cache.publish_file(output, artefact).map_err(|error| {
            super::cache_failure(portable, "Cannot publish the handler artifact", error)
        })
    }

    fn compile_arguments(home: &GroovyHome) -> Vec<String> {
        vec![
            String::from("-cp"),
            home.class_path(),
            String::from(Self::COMPILER_CLASS),
            String::from("-j"),
            String::from("-d"),
            String::from(Self::CLASSES_FOLDER),
            String::from(Self::JAVA_FILE),
            String::from(Self::GROOVY_FILE),
        ]
    }

    fn jar_arguments() -> Vec<String> {
        vec![
            String::from("--create"),
            String::from("--file"),
            String::from(Self::OUTPUT_FILE),
            String::from("--main-class"),
            String::from(Self::MAIN_CLASS),
            String::from("-C"),
            String::from(Self::CLASSES_FOLDER),
            String::from("."),
        ]
    }

    fn add_runtime(runtime_jar: &Path, classes: &Path, portable: &str) -> Result<(), Failure> {
        let arguments = vec![
            String::from("xf"),
            runtime_jar.to_string_lossy().into_owned(),
        ];
        Self::run(Self::JAR, &arguments, classes, portable)?;
        Self::remove_runtime_manifest(classes, portable)
    }

    fn remove_runtime_manifest(classes: &Path, portable: &str) -> Result<(), Failure> {
        let [folder, file] = Self::RUNTIME_MANIFEST;
        let manifest = classes.join(folder).join(file);
        if !manifest.exists() {
            return Ok(());
        }
        std::fs::remove_file(&manifest).map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot prepare the Groovy runtime classes: {error}"),
            )
        })
    }

    fn run(
        program: &str,
        arguments: &[String],
        folder: &Path,
        portable: &str,
    ) -> Result<(), Failure> {
        let output = bounded_compiler_output(program, arguments, Some(folder))
            .map_err(|error| Self::start_failure(program, &error, portable))?;
        Self::check_overflow(&output, portable)?;
        Self::check_status(&output, portable)
    }

    fn start_failure(program: &str, error: &str, portable: &str) -> Failure {
        if names_a_missing_program(error) {
            return missing_compiler(program, portable);
        }
        invalid_source(portable, &format!("Cannot start '{program}': {error}"))
    }

    fn check_overflow(output: &CompilerOutput, portable: &str) -> Result<(), Failure> {
        if output.overflow {
            return Err(invalid_source(
                portable,
                "Compiler output exceeded the 1 MiB diagnostic limit.",
            ));
        }
        Ok(())
    }

    fn check_status(output: &CompilerOutput, portable: &str) -> Result<(), Failure> {
        if output.status.success() {
            return Ok(());
        }
        let detail = String::from_utf8_lossy(&output.stderr);
        Err(invalid_source(
            portable,
            &format!("Handler failed to compile: {}", Self::error_line(&detail)),
        ))
    }

    fn error_line(detail: &str) -> &str {
        Self::source_line(detail)
            .unwrap_or_else(|| super::CompilerErrorLine::read("groovy", detail))
    }

    fn source_line(detail: &str) -> Option<&str> {
        detail
            .lines()
            .find_map(|line| Self::names_a_source(line).then(|| line.trim()))
    }

    fn names_a_source(line: &str) -> bool {
        Self::SOURCE_NAMES.iter().any(|name| line.contains(name))
    }
}
