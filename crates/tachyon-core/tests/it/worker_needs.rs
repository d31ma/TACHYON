#![allow(clippy::expect_used)]

use crate::support::program_probe::ProgramProbe;
use crate::support::published_files::PublishedFiles;
use std::path::Path;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
    ProjectDiscovery,
};

struct NeedingWorker {
    extension: &'static str,
    program: &'static str,
    flag: &'static str,
}

impl NeedingWorker {
    const ANSWER: &'static str = "42";
    const SECTION: &'static str = "docs/annotations/scheduling-yon";
    const WORKER: &'static str = "server/workers/settle";
    const SERVICE: &'static str = "server/services/LedgerService";
    const VIEW: &'static str = "client/pages/tac.html";

    const CPP: Self = Self {
        extension: "hpp",
        program: "clang++",
        flag: "--version",
    };

    const JAVASCRIPT: Self = Self {
        extension: "js",
        program: "node",
        flag: "--version",
    };

    const TYPESCRIPT: Self = Self {
        extension: "ts",
        program: "bun",
        flag: "--version",
    };

    const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        flag: "--version",
    };

    const PHP: Self = Self {
        extension: "php",
        program: "php-cgi",
        flag: "--version",
    };

    const JAVA: Self = Self {
        extension: "java",
        program: "javac",
        flag: "--version",
    };

    const GROOVY: Self = Self {
        extension: "groovy",
        program: "groovyc",
        flag: "--version",
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        flag: "-version",
    };

    const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        flag: "--version",
    };

    const FSHARP: Self = Self {
        extension: "fs",
        program: "dotnet",
        flag: "--version",
    };

    const RUST: Self = Self {
        extension: "rs",
        program: "cargo",
        flag: "--version",
    };

    const DART: Self = Self {
        extension: "dart",
        program: "dart",
        flag: "--version",
    };

    const GO: Self = Self {
        extension: "go",
        program: "go",
        flag: "version",
    };

    const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        flag: "--version",
    };

    const LUA: Self = Self {
        extension: "lua",
        program: "lua",
        flag: "-v",
    };

    async fn assert_gets_its_service(&self) {
        if self.skipped() {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let worker = format!("{}.{}", Self::WORKER, self.extension);
        let service = format!("{}.{}", Self::SERVICE, self.extension);
        Self::copy(root.path(), &service);
        Self::copy(root.path(), &worker);
        Self::write_view(root.path());
        ProjectDiscovery::discover(root.path())
            .unwrap_or_else(|failure| panic!("{}: {failure}", self.extension));
        let body = self.invoke(root.path(), &worker).await;
        assert_eq!(body.trim(), Self::ANSWER, "{}: {body}", self.extension);
    }

    fn skipped(&self) -> bool {
        if ProgramProbe::answers(self.program, self.flag) {
            return false;
        }
        assert!(
            std::env::var_os("CI").is_none(),
            "{} is required in CI",
            self.program
        );
        eprintln!(
            "skipping {}: {} is unavailable",
            self.extension, self.program
        );
        true
    }

    fn write_view(root: &Path) {
        let page = root.join(Self::VIEW);
        std::fs::create_dir_all(page.parent().expect("parent")).expect("directories");
        std::fs::write(page, "<app-view />").expect("view");
    }

    fn copy(root: &Path, relative: &str) {
        let published = PublishedFiles::folder(Self::SECTION).join(relative);
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directories");
        std::fs::copy(published, path).expect("published example");
    }

    async fn invoke(&self, root: &Path, worker: &str) -> String {
        let source = HandlerSource::discover(root, worker)
            .unwrap_or_else(|failure| panic!("{}: {failure}", self.extension));
        let mut options = HandlerSupervisorOptions {
            default_timeout: Duration::from_mins(2),
            ..HandlerSupervisorOptions::default()
        };
        if self.extension == "ts" {
            options.runtimes.javascript = "bun".into();
        }
        let supervisor = HandlerSupervisor::new(options).expect("supervisor");
        let request = HandlerRequest::route("reconcile", "/", HttpMethod::Post);
        let response = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("invocation");
        assert!(
            response.error.is_none(),
            "{}: {:?}",
            self.extension,
            response.error
        );
        response.body.map(|body| body.data).unwrap_or_default()
    }
}

#[tokio::test]
async fn a_javascript_worker_gets_its_needed_service() {
    NeedingWorker::JAVASCRIPT.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_typescript_worker_gets_its_needed_service() {
    NeedingWorker::TYPESCRIPT.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_python_worker_gets_its_needed_service() {
    NeedingWorker::PYTHON.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_php_worker_gets_its_needed_service() {
    NeedingWorker::PHP.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_java_worker_gets_its_needed_service() {
    NeedingWorker::JAVA.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_groovy_worker_gets_its_needed_service() {
    NeedingWorker::GROOVY.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_kotlin_worker_gets_its_needed_service() {
    NeedingWorker::KOTLIN.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_csharp_worker_gets_its_needed_service() {
    NeedingWorker::CSHARP.assert_gets_its_service().await;
}

#[tokio::test]
async fn an_fsharp_worker_gets_its_needed_service() {
    NeedingWorker::FSHARP.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_rust_worker_gets_its_needed_service() {
    NeedingWorker::RUST.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_dart_worker_gets_its_needed_service() {
    NeedingWorker::DART.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_go_worker_gets_its_needed_service() {
    NeedingWorker::GO.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_ruby_worker_gets_its_needed_service() {
    NeedingWorker::RUBY.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_lua_worker_gets_its_needed_service() {
    NeedingWorker::LUA.assert_gets_its_service().await;
}

#[tokio::test]
async fn a_cpp_worker_gets_its_needed_service() {
    NeedingWorker::CPP.assert_gets_its_service().await;
}
