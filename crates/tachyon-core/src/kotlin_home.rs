use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The Kotlin compilers ship for Windows only as `.bat` launchers, which a spawn cannot run, so
/// on Windows Tachyon starts the JVM those launchers start instead.
pub(crate) struct KotlinHome {
    library: PathBuf,
}

impl KotlinHome {
    const PATH_VARIABLE: &'static str = "PATH";
    const JAVA_HOME_VARIABLE: &'static str = "JAVA_HOME";
    const JAVA: &'static str = "java";
    const LIBRARY_FOLDER: &'static str = "lib";
    const PRELOADER_JAR: &'static str = "kotlin-preloader.jar";
    const COMPILER_JAR: &'static str = "kotlin-compiler.jar";
    const PRELOADER: &'static str = "org.jetbrains.kotlin.preloading.Preloader";
    const COMPILERS: [(&'static str, &'static str); 2] = [
        ("kotlinc", "org.jetbrains.kotlin.cli.jvm.K2JVMCompiler"),
        ("kotlinc-js", "org.jetbrains.kotlin.cli.js.K2JSCompiler"),
    ];
    /// The options `kotlinc.bat` gives a Java 9 or newer runtime.
    const JVM_OPTIONS: [&'static str; 4] = [
        "-Xmx512M",
        "-Xms128M",
        "--add-opens",
        "java.base/java.util=ALL-UNNAMED",
    ];

    /// A command for `program`; a Kotlin compiler reached only through its Windows launcher runs
    /// on the JVM directly, and every other program is spawned as named.
    pub(crate) fn command(program: impl AsRef<OsStr>) -> Command {
        let program = program.as_ref();
        let search_path = std::env::var_os(Self::PATH_VARIABLE).unwrap_or_default();
        let launched = cfg!(windows)
            .then(|| Self::launcher_command(&search_path, program))
            .flatten();
        launched.unwrap_or_else(|| Command::new(program))
    }

    fn launcher_command(search_path: &OsStr, program: &OsStr) -> Option<Command> {
        let main_class = Self::main_class(program)?;
        let home = Self::behind_launcher(search_path, program)?;
        Some(home.java_command(main_class))
    }

    fn main_class(program: &OsStr) -> Option<&'static str> {
        Self::COMPILERS
            .iter()
            .find(|(name, _)| OsStr::new(name) == program)
            .map(|(_, main_class)| *main_class)
    }

    fn behind_launcher(search_path: &OsStr, program: &OsStr) -> Option<Self> {
        let executable = Self::named(program, ".exe");
        let launcher = Self::named(program, ".bat");
        for folder in std::env::split_paths(search_path) {
            if folder.join(&executable).is_file() {
                return None;
            }
            if folder.join(&launcher).is_file() {
                return folder.parent().and_then(Self::at);
            }
        }
        None
    }

    fn named(program: &OsStr, extension: &str) -> OsString {
        let mut name = program.to_os_string();
        name.push(extension);
        name
    }

    fn at(home: &Path) -> Option<Self> {
        let library = home.join(Self::LIBRARY_FOLDER);
        let complete = [Self::PRELOADER_JAR, Self::COMPILER_JAR]
            .iter()
            .all(|jar| library.join(jar).is_file());
        complete.then_some(Self { library })
    }

    fn java_command(&self, main_class: &str) -> Command {
        let mut command = Command::new(Self::java());
        command
            .args(Self::JVM_OPTIONS)
            .arg("-cp")
            .arg(self.library.join(Self::PRELOADER_JAR))
            .arg(Self::PRELOADER)
            .arg("-cp")
            .arg(self.library.join(Self::COMPILER_JAR))
            .arg(main_class);
        command
    }

    fn java() -> PathBuf {
        let java = format!("{}{}", Self::JAVA, std::env::consts::EXE_SUFFIX);
        let from_home = std::env::var_os(Self::JAVA_HOME_VARIABLE)
            .map(|home| PathBuf::from(home).join("bin").join(&java));
        from_home
            .filter(|path| path.is_file())
            .unwrap_or_else(|| PathBuf::from(Self::JAVA))
    }
}

#[cfg(test)]
mod tests {
    use super::KotlinHome;
    use std::ffi::OsStr;
    use std::path::Path;

    fn install(home: &Path, launchers: &[&str]) -> std::io::Result<()> {
        let bin = home.join("bin");
        let library = home.join("lib");
        std::fs::create_dir_all(&bin)?;
        std::fs::create_dir_all(&library)?;
        for launcher in launchers {
            std::fs::write(bin.join(launcher), b"@echo off")?;
        }
        for jar in ["kotlin-preloader.jar", "kotlin-compiler.jar"] {
            std::fs::write(library.join(jar), b"jar")?;
        }
        Ok(())
    }

    fn search_path(folders: &[&Path]) -> std::io::Result<std::ffi::OsString> {
        std::env::join_paths(folders).map_err(std::io::Error::other)
    }

    #[test]
    fn a_compiler_behind_its_windows_launcher_runs_on_the_jvm() -> std::io::Result<()> {
        let home = tempfile::tempdir()?;
        install(home.path(), &["kotlinc.bat", "kotlinc-js.bat"])?;
        let path = search_path(&[&home.path().join("bin")])?;
        for (program, main_class) in [
            ("kotlinc", "org.jetbrains.kotlin.cli.jvm.K2JVMCompiler"),
            ("kotlinc-js", "org.jetbrains.kotlin.cli.js.K2JSCompiler"),
        ] {
            let command =
                KotlinHome::launcher_command(&path, OsStr::new(program)).ok_or_else(|| {
                    std::io::Error::other(format!("{program} has no launcher command"))
                })?;
            let arguments: Vec<_> = command.get_args().collect();
            assert_eq!(arguments.last(), Some(&OsStr::new(main_class)), "{program}");
            let compiler_jar = home.path().join("lib").join("kotlin-compiler.jar");
            assert!(
                arguments.contains(&compiler_jar.as_os_str()),
                "{arguments:?}"
            );
            assert!(
                arguments.contains(&OsStr::new("org.jetbrains.kotlin.preloading.Preloader")),
                "{arguments:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn executables_incomplete_homes_and_other_programs_are_spawned_as_named() -> std::io::Result<()>
    {
        let executable = tempfile::tempdir()?;
        std::fs::write(executable.path().join("kotlinc.exe"), b"exe")?;
        let home = tempfile::tempdir()?;
        install(home.path(), &["kotlinc.bat"])?;
        let bin = home.path().join("bin");
        let executable_first = search_path(&[executable.path(), &bin])?;
        assert!(KotlinHome::launcher_command(&executable_first, OsStr::new("kotlinc")).is_none());
        let launcher_only = search_path(&[&bin])?;
        assert!(KotlinHome::launcher_command(&launcher_only, OsStr::new("javac")).is_none());
        std::fs::remove_file(home.path().join("lib").join("kotlin-preloader.jar"))?;
        assert!(KotlinHome::launcher_command(&launcher_only, OsStr::new("kotlinc")).is_none());
        Ok(())
    }
}
