use super::{AppFeature, AppTarget, BaseFiles, ProjectChoices, YonLanguage};

pub(crate) struct ServerFiles;

impl ServerFiles {
    pub(crate) const APPLICATION: &'static str = "server/application.json";
    pub(crate) const DEFAULT_ENV: &'static str = "# Copy to .env and add only server secrets. Never commit .env.\n\
# YON_DATASOURCE_PASSWORD=replace-me\n";
    const SERVER_HOST: &'static str = "  \"server\": {\n    \"host\": \"127.0.0.1\",\n";
    const SERVER_PORT: &'static str = "    \"port\": 8080\n  }";
    const DATASOURCE: &'static str = ",\n  \"datasource\": {\n    \"driver\": \"sqlite\",\n    \"url\": \"sqlite::memory:\"\n  }";

    pub(crate) fn for_choices(choices: &ProjectChoices) -> Vec<(String, String)> {
        let mut files = Self::fixed_files(
            choices.features.contains(&AppFeature::Storage),
            choices.server_uses_typescript(),
            &AppTarget::app_origins(&choices.targets),
        );
        files.extend(
            choices
                .yon
                .iter()
                .flat_map(|language| Self::language_files(*language)),
        );
        files
    }

    pub(crate) fn language_files(language: YonLanguage) -> Vec<(String, String)> {
        let mut files = language.files();
        files.extend(Self::javascript_files(language));
        files
    }

    pub(crate) fn application(storage: bool, origins: &[&str]) -> String {
        let datasource = Self::datasource(storage);
        let origins = Self::origins(origins);
        format!(
            "{{\n{}{origins}{}{datasource}\n}}\n",
            Self::SERVER_HOST,
            Self::SERVER_PORT
        )
    }

    fn origins(origins: &[&str]) -> String {
        if origins.is_empty() {
            return String::new();
        }
        let listed: Vec<String> = origins
            .iter()
            .map(|origin| format!("      \"{origin}\""))
            .collect();
        format!("    \"origins\": [\n{}\n    ],\n", listed.join(",\n"))
    }

    fn fixed_files(
        storage: bool,
        typescript_only: bool,
        origins: &[&str],
    ) -> Vec<(String, String)> {
        vec![
            (
                String::from(".env.example"),
                String::from(Self::DEFAULT_ENV),
            ),
            (
                String::from(Self::APPLICATION),
                Self::application(storage, origins),
            ),
            (
                String::from("server/jsconfig.json"),
                BaseFiles::jsconfig("server", typescript_only),
            ),
            (String::from("server/data/.gitkeep"), String::new()),
            (String::from("server/deps/.gitkeep"), String::new()),
        ]
    }

    fn datasource(storage: bool) -> &'static str {
        if storage {
            return Self::DATASOURCE;
        }
        ""
    }

    fn javascript_files(language: YonLanguage) -> Vec<(String, String)> {
        if language != YonLanguage::JavaScript {
            return Vec::new();
        }
        Self::home_files()
    }

    fn home_files() -> Vec<(String, String)> {
        vec![
            (
                String::from("server/controllers/HomeController.js"),
                String::from(
                    "@RequestRoute(\"/\")\n@Public\nexport class HomeController {\n  @GetRoute\n  async get() {\n    return { ok: true, framework: 'Tachyon' }\n  }\n}\n",
                ),
            ),
            (
                String::from("server/tests/HomeTestSetup.js"),
                String::from(
                    "@TestSetup\nexport default class HomeTestSetup {\n  static configure() {\n    return {}\n  }\n}\n",
                ),
            ),
            (
                String::from("server/tests/HomeControllerTest.js"),
                String::from(
                    "@Test(HomeTestSetup, {\"partition\":\"tests\"})\nexport default class HomeControllerTest {\n  async testGetSaysOk() {\n    const body = await new HomeController().get()\n    Assert.isTrue(body.ok)\n  }\n}\n",
                ),
            ),
        ]
    }
}
