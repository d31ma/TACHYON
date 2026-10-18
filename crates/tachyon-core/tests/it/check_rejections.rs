#![allow(clippy::expect_used)]

use crate::support::json_http::JsonHttp;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct CheckLanguage {
    extension: &'static str,
    programs: &'static [&'static str],
}

impl CheckLanguage {
    const ALL: [Self; 7] = [
        Self {
            extension: "java",
            programs: &["java", "javac"],
        },
        Self {
            extension: "groovy",
            programs: &["java", "groovyc"],
        },
        Self {
            extension: "hpp",
            programs: &["clang++"],
        },
        Self {
            extension: "cs",
            programs: &["dotnet"],
        },
        Self {
            extension: "fs",
            programs: &["dotnet"],
        },
        Self {
            extension: "js",
            programs: &["node"],
        },
        Self {
            extension: "ts",
            programs: &["node"],
        },
    ];

    fn available(&self) -> bool {
        let found = self.programs.iter().all(|program| Self::runs(program));
        if !found {
            assert!(
                std::env::var_os("CI").is_none(),
                "{} toolchain is required in CI",
                self.extension
            );
            eprintln!(
                "skipping {} check rejections: missing one of {:?}",
                self.extension, self.programs
            );
        }
        found
    }

    fn runs(program: &str) -> bool {
        let flag = if program == "java" || program == "javac" {
            "-version"
        } else {
            "--version"
        };
        Command::new(program)
            .arg(flag)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn write_project(&self, root: &Path) {
        for role in ["models", "services", "controllers"] {
            let source = CheckRejections::examples().join(role);
            let target = root.join("server").join(role);
            std::fs::create_dir_all(&target).expect("role folder");
            for entry in std::fs::read_dir(&source).expect("examples") {
                let path = entry.expect("example").path();
                let matches = path
                    .extension()
                    .is_some_and(|found| found == self.extension);
                if matches {
                    let name = path.file_name().expect("file name");
                    std::fs::copy(&path, target.join(name)).expect("copy example");
                }
            }
        }
    }

    async fn answers(&self) -> Vec<String> {
        let root = tempfile::tempdir().expect("project");
        self.write_project(root.path());
        let rejected = CheckRejections::rejected();
        let mut bodies = vec![CheckRejections::ACCEPTED];
        for (body, _) in &rejected {
            bodies.push(body.as_str());
        }
        JsonHttp::post_all(root.path(), "/orders", &bodies).await
    }
}

struct CheckRejections;

impl CheckRejections {
    const ACCEPTED: &'static str = r#"{"name":"order","level":3}"#;

    fn examples() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/content/docs/annotations/validation-yon/server")
    }

    fn rejected() -> Vec<(String, String)> {
        let long_name = "a".repeat(81);
        vec![
            (
                String::from(r#"{"name":"order","level":null}"#),
                Self::errors(&[("level", "Required")]),
            ),
            (
                String::from(r#"{"name":"","level":0}"#),
                Self::errors(&[("level", "Min"), ("name", "NotEmpty"), ("name", "Length")]),
            ),
            (
                String::from(r#"{"name":"   ","level":0}"#),
                Self::errors(&[("level", "Min"), ("name", "NotEmpty")]),
            ),
            (
                format!(r#"{{"name":"{long_name}","level":10}}"#),
                Self::errors(&[("level", "Max"), ("name", "Length")]),
            ),
        ]
    }

    fn errors(pairs: &[(&str, &str)]) -> String {
        let entries: Vec<String> = pairs
            .iter()
            .map(|(field, rule)| format!(r#"{{"field":"{field}","rule":"{rule}"}}"#))
            .collect();
        format!(
            r#"{{"type":"about:blank","title":"Invalid input","status":422,"errors":[{}],"truncated":false}}"#,
            entries.join(",")
        )
    }

    fn body(extension: &str, response: &str) -> String {
        let (head, body) = response
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("{extension}: {response}"));
        let status_line = head.lines().next().unwrap_or_default();
        let content_type = head.contains("content-type: application/problem+json");
        assert!(
            status_line == "HTTP/1.1 422 Unprocessable Entity" && content_type,
            "{extension}: {response}"
        );
        body.to_owned()
    }

    fn without_reference(extension: &str, body: &str) -> String {
        let answered: serde_json::Value =
            serde_json::from_str(body).unwrap_or_else(|_| panic!("{extension}: {body}"));
        let reference = answered["reference"].as_str().unwrap_or_default();
        assert!(!reference.is_empty(), "{extension}: {body}");
        body.replace(&format!(r#","reference":"{reference}""#), "")
    }

    fn check(extension: &str, answers: &[String]) -> Vec<String> {
        let (accepted, rejected) = answers.split_first().expect("answers");
        assert!(
            accepted.starts_with("HTTP/1.1 200 OK"),
            "{extension}: {accepted}"
        );
        rejected
            .iter()
            .map(|response| Self::without_reference(extension, &Self::body(extension, response)))
            .collect()
    }
}

#[tokio::test]
async fn every_language_answers_the_same_problem_for_the_same_invalid_input() {
    let mut running = tokio::task::JoinSet::new();
    for language in CheckLanguage::ALL {
        if language.available() {
            running.spawn(async move { (language.extension, language.answers().await) });
        }
    }
    let expected: Vec<String> = CheckRejections::rejected()
        .into_iter()
        .map(|(_, errors)| errors)
        .collect();
    while let Some(finished) = running.join_next().await {
        let (extension, answers) = finished.expect("language task");
        let bodies = CheckRejections::check(extension, &answers);
        assert_eq!(bodies, expected, "{extension}");
    }
}
