use super::model_files::ModelParser;
use super::{Model, WebEventModels, WebEventSources};
use serde_json::json;
use std::path::Path;

struct Preludes;

impl Preludes {
    const LISTENERS: [(&str, &str); 8] = [
        (
            "ts",
            "class ProbeService {\n@OnEvent\nchanged(event: ConnectivityChanged) {}\n}\n",
        ),
        (
            "js",
            "class ProbeService {\n/** @param {ConnectivityChanged} event */\n@OnEvent\nchanged(event) {}\n}\n",
        ),
        (
            "rs",
            "#[derive(Default)]\nstruct ProbeService {}\nimpl ProbeService {\n    #[OnEvent]\n    fn changed(&mut self, event: ConnectivityChanged) {}\n}\n",
        ),
        (
            "go",
            "// @Service\ntype ProbeService struct{}\n\n// @OnEvent\nfunc (probe *ProbeService) Changed(event ConnectivityChanged) {\n}\n",
        ),
        (
            "cs",
            "[Service]\npublic class ProbeService\n{\n    [OnEvent]\n    public void Changed(ConnectivityChanged changed) { }\n}\n",
        ),
        (
            "dart",
            "@Service()\nclass ProbeService {\n  @OnEvent\n  void changed(ConnectivityChanged event) {}\n}\n",
        ),
        (
            "swift",
            "@Service\nfinal class ProbeService {\n    @OnEvent\n    func changed(_ event: ConnectivityChanged) {}\n}\n",
        ),
        (
            "rb",
            "#[Service]\nclass ProbeService\n  # @param event [ConnectivityChanged]\n  #[OnEvent]\n  def changed(event)\n  end\nend\n",
        ),
    ];

    fn field_names(model: &Model) -> Vec<String> {
        let Model::Record(record) = model else {
            panic!("{model:?} is not a record");
        };
        record
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect()
    }

    fn listener_models(
        extension: &str,
        source: &str,
        realm: &str,
    ) -> Result<serde_json::Value, String> {
        let root = tempfile::tempdir().expect("project");
        let path_text = format!("{realm}/services/ProbeService.{extension}");
        let path = Path::new(&path_text);
        let mut module = crate::events::extract(path, source, "tac")
            .expect("parse")
            .expect("events");
        crate::events::describe(&mut module, path, root.path(), realm)
            .map_err(|failure| failure.to_string())?;
        let listener = &module.types[0].listeners[0];
        Ok(listener.models.clone().expect("models"))
    }
}

#[test]
fn every_language_declares_the_web_event_models_with_their_fields() {
    let expected = WebEventModels::models();
    let sources = [
        ("rs", WebEventSources::rust()),
        ("go", WebEventSources::go()),
        ("cs", WebEventSources::csharp()),
        ("dart", WebEventSources::dart()),
        ("rb", WebEventSources::ruby()),
        (
            "swift",
            WebEventSources::swift_used_by(&WebEventModels::names().join(" ")),
        ),
    ];
    for (extension, source) in sources {
        let declared = ModelParser::models(extension, &source);
        assert_eq!(declared.len(), expected.len(), "{extension}: {declared:?}");
        for (name, model) in &expected {
            let found = declared
                .iter()
                .find(|(declared_name, _)| declared_name == name)
                .unwrap_or_else(|| panic!("{extension} does not declare {name}"));
            assert_eq!(
                Preludes::field_names(&found.1),
                Preludes::field_names(model),
                "{extension} {name}"
            );
        }
    }
}

#[test]
fn fsharp_declares_the_web_event_models_as_records_everyone_on_the_page_can_name() {
    let source = WebEventSources::fsharp();
    assert!(source.starts_with("namespace global\n"), "{source}");
    assert!(source.contains("\ntype ConnectivityChanged = { Online: bool }\n"));
    assert!(source.contains(
        "\ntype PositionFound = { Latitude: float; Longitude: float; Accuracy: float; Timestamp: float }\n"
    ));
    assert!(source.contains("\ntype ShareFinished() = class end\n"));
    for name in WebEventModels::names() {
        assert!(source.contains(&format!("\ntype {name}")), "{name}");
    }
}

#[test]
fn listeners_in_every_client_language_accept_the_web_event_models() {
    for (extension, source) in Preludes::LISTENERS {
        let models = Preludes::listener_models(extension, source, "client")
            .unwrap_or_else(|error| panic!("{extension}: {error}"));
        assert_eq!(
            models,
            json!({"ConnectivityChanged": {"open": false, "fields": {"online": "!boolean"}}}),
            "{extension}"
        );
    }
}

#[test]
fn server_listeners_do_not_get_the_web_event_models() {
    let (extension, source) = Preludes::LISTENERS[0];
    let error = Preludes::listener_models(extension, source, "server").expect_err("server");
    assert!(
        error.contains("'ConnectivityChanged' is not a model"),
        "{error}"
    );
}
