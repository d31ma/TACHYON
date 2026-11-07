#![allow(clippy::expect_used)]
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
};

struct PublishedConverters;

impl PublishedConverters {
    const FOLDERS: [&'static str; 3] = ["models", "converters", "controllers"];

    fn section(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/content/docs/annotations")
            .join(name)
            .join("server")
    }

    fn project(section: &str, extension: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        let published = Self::section(section);
        for folder in Self::FOLDERS {
            let target = root.path().join("server").join(folder);
            fs::create_dir_all(&target).expect("folder");
            for entry in fs::read_dir(published.join(folder)).expect("published folder") {
                let path = entry.expect("published file").path();
                if path.extension().and_then(|found| found.to_str()) == Some(extension) {
                    fs::copy(&path, target.join(path.file_name().expect("name")))
                        .expect("copied file");
                }
            }
        }
        root
    }

    async fn answer(section: &str, extension: &str) -> serde_json::Value {
        let root = Self::project(section, extension);
        let relative = format!("server/controllers/CatalogController.{extension}");
        let source = HandlerSource::discover(root.path(), &relative)
            .unwrap_or_else(|failure| panic!("{section} {extension}: {failure}"));
        let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
            default_timeout: Duration::from_mins(2),
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor");
        let answer = supervisor
            .invoke(
                &source,
                &HandlerRequest::route("published-converter", "/catalog", HttpMethod::Get),
                &HandlerCancellation::default(),
            )
            .await
            .expect("invoke");
        assert!(
            answer.error.is_none(),
            "{section} {extension}: {:?}",
            answer.error
        );
        serde_json::from_str(&answer.body.expect("body").data).expect("JSON")
    }

    async fn assert_field_plan(extension: &str) {
        let actual = Self::answer("mapping-yon", extension).await;
        let summary = actual["summary"].as_object().expect("summary object");
        assert!(!summary.contains_key("extra"), "{extension}: {actual}");
        assert!(!summary.contains_key("detail"), "{extension}: {actual}");
        assert_eq!(
            summary.get("absent"),
            Some(&serde_json::Value::Null),
            "{extension}: {actual}"
        );
        assert_eq!(summary["kind"], "product", "{extension}: {actual}");
        assert_eq!(
            actual,
            serde_json::json!({"summary":{"productId":7,"title":"Anvil","absent":null,"kind":"product"}}),
            "{extension}"
        );
    }

    async fn assert_typed_models(extension: &str) {
        let actual = Self::answer("typed-mapping-yon", extension).await;
        assert!(
            !actual.to_string().contains("private"),
            "{extension}: {actual}"
        );
        assert_eq!(
            actual,
            serde_json::json!({"id":7,"name":"Anvil","detail":{"name":"detail"},"items":[{"name":"detail"}],"kind":"product"}),
            "{extension}"
        );
    }
}

#[tokio::test]
async fn published_java_converter_maps_the_field_plan() {
    PublishedConverters::assert_field_plan("java").await;
}

#[tokio::test]
async fn published_groovy_converter_maps_the_field_plan() {
    PublishedConverters::assert_field_plan("groovy").await;
}

#[tokio::test]
async fn published_kotlin_converter_maps_the_field_plan() {
    PublishedConverters::assert_field_plan("kt").await;
}

#[tokio::test]
async fn published_ruby_converter_maps_the_field_plan() {
    PublishedConverters::assert_field_plan("rb").await;
}

#[tokio::test]
async fn published_java_converter_maps_typed_models() {
    PublishedConverters::assert_typed_models("java").await;
}

#[tokio::test]
async fn published_groovy_converter_maps_typed_models() {
    PublishedConverters::assert_typed_models("groovy").await;
}

#[tokio::test]
async fn published_kotlin_converter_maps_typed_models() {
    PublishedConverters::assert_typed_models("kt").await;
}

#[tokio::test]
async fn published_ruby_converter_maps_typed_models() {
    PublishedConverters::assert_typed_models("rb").await;
}
