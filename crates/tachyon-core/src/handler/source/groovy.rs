use super::{
    CacheDirectory, Compiler, CompilerOutput, EntryClass, Failure, PreparedCommand,
    bounded_compiler_output, build_once_under_lock, compiled_digest, compiler_identity,
    compiler_workspace, invalid_source, missing_compiler, names_a_missing_program,
    prepare_cached_artifact, protocol_prelude, valid_owned_artifact, write_compiler_file,
};
use crate::groovy_home::GroovyHome;
use crate::groovy_source::GroovySource;
use std::path::{Path, PathBuf};

pub(super) struct GroovyHandler;

struct GroovyBuild<'build> {
    groovy: &'build str,
    java: &'build str,
    home: &'build GroovyHome,
    runtime_jar: &'build Path,
}

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
        let identity = format!(
            "{}{}",
            runtime_jar.to_string_lossy(),
            compiler_identity(Self::JAVA)
        );
        let digest = compiled_digest(
            groovy.as_bytes(),
            java.as_bytes(),
            &identity,
            std::env::consts::ARCH,
            std::env::consts::OS,
        );
        let artefact = PathBuf::from(format!("{digest}{}", Self::COMPILER.suffix));
        if !cache.is_valid_file(&artefact) {
            let build = GroovyBuild {
                groovy,
                java: &java,
                home: &home,
                runtime_jar: &runtime_jar,
            };
            build_once_under_lock(cache, &artefact, &digest, portable, || {
                Self::build(&build, &artefact, portable, cache)
            })?;
        }
        prepare_cached_artifact(&Self::COMPILER, &artefact, &digest, portable, cache)
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
        write_compiler_file(
            &folder.join(Self::GROOVY_FILE),
            build.groovy.as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &folder.join(Self::JAVA_FILE),
            build.java.as_bytes(),
            portable,
        )?;
        let classes = folder.join(Self::CLASSES_FOLDER);
        std::fs::create_dir(&classes).map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot create the compiled classes folder: {error}"),
            )
        })?;
        Self::run(
            Self::JAVA,
            &Self::compile_arguments(build.home),
            folder,
            portable,
        )?;
        Self::add_runtime(build.runtime_jar, &classes, portable)?;
        let output = folder.join(Self::OUTPUT_FILE);
        Self::run(Self::JAR, &Self::jar_arguments(), folder, portable)?;
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
        let [folder, file] = Self::RUNTIME_MANIFEST;
        let manifest = classes.join(folder).join(file);
        if manifest.exists() {
            std::fs::remove_file(&manifest).map_err(|error| {
                invalid_source(
                    portable,
                    &format!("Cannot prepare the Groovy runtime classes: {error}"),
                )
            })?;
        }
        Ok(())
    }

    fn run(
        program: &str,
        arguments: &[String],
        folder: &Path,
        portable: &str,
    ) -> Result<(), Failure> {
        let output = match bounded_compiler_output(program, arguments, Some(folder)) {
            Ok(output) => output,
            Err(error) if names_a_missing_program(&error) => {
                return Err(missing_compiler(program, portable));
            }
            Err(error) => {
                return Err(invalid_source(
                    portable,
                    &format!("Cannot start '{program}': {error}"),
                ));
            }
        };
        Self::check(&output, portable)
    }

    fn check(output: &CompilerOutput, portable: &str) -> Result<(), Failure> {
        if output.overflow {
            return Err(invalid_source(
                portable,
                "Compiler output exceeded the 1 MiB diagnostic limit.",
            ));
        }
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
        for line in detail.lines() {
            let names_a_source = Self::SOURCE_NAMES.iter().any(|name| line.contains(name));
            if names_a_source {
                return line.trim();
            }
        }
        super::compiler_error_line(detail)
    }
}
