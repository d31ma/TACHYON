use super::{AppFeature, AppName, BaseFiles, Choice, PageFile, ProjectChoices, TacLanguage};
use crate::tac_python::RuntimeAssets;
use std::collections::BTreeMap;

pub(crate) struct ClientFiles;

impl ClientFiles {
    const SHELL_VIEW: &'static str = "client/components/app/shell/tac.html";
    const SHELL_BRAND: &'static str = "{shell.brand}";
    const PLAIN_BRAND: &'static str = "Tachyon";
    const ICON_PATH: &'static str = "client/shared/assets/icon.svg";
    const MANIFEST_PATH: &'static str = "client/manifest.json";
    const ICON_IMAGES: [(&'static str, &'static [u8]); 2] = [
        (
            "client/shared/assets/icon-192.png",
            include_bytes!("icons/icon-192.png"),
        ),
        (
            "client/shared/assets/icon-512.png",
            include_bytes!("icons/icon-512.png"),
        ),
    ];
    const ICON: &'static str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\">\n  <rect width=\"64\" height=\"64\" rx=\"14\" fill=\"#1d4ed8\"/>\n  <path d=\"M18 18h28v8H36v22h-8V26H18z\" fill=\"#e2e8f0\"/>\n</svg>\n";

    pub(crate) fn images_for(
        files: &BTreeMap<String, String>,
    ) -> &'static [(&'static str, &'static [u8])] {
        [&[][..], &Self::ICON_IMAGES[..]][usize::from(files.contains_key(Self::MANIFEST_PATH))]
    }

    pub(crate) fn for_choices(app_name: &str, choices: &ProjectChoices) -> Vec<(String, String)> {
        let javascript_name = AppName::javascript_literal(app_name);
        let mut files = Self::fixed_files(&javascript_name, choices);
        files.push((
            String::from(Self::SHELL_VIEW),
            Self::shell_view(&choices.tac),
        ));
        files.push((
            String::from(PageFile::PATH),
            PageFile::text(&Self::components(choices)),
        ));
        files.extend(
            choices
                .tac
                .iter()
                .flat_map(|language| Self::language_files(*language, &javascript_name)),
        );
        files
    }

    pub(crate) fn language_files(
        language: TacLanguage,
        javascript_name: &str,
    ) -> Vec<(String, String)> {
        let mut files = Self::template_files(language);
        files.extend(Self::javascript_files(language, javascript_name));
        files.extend(Self::greeting_files(language));
        files.extend(language.editor_files());
        files
    }

    pub(crate) fn greeting_tag(language: TacLanguage) -> Option<String> {
        Self::has_greeting(language).then(|| format!("greeting-{}", language.slug()))
    }

    pub(crate) fn feature_tag(feature: AppFeature) -> Option<String> {
        let tag = feature.component();
        (!tag.is_empty()).then(|| String::from(tag))
    }

    fn components(choices: &ProjectChoices) -> Vec<String> {
        let greetings = choices.tac.iter().copied().filter_map(Self::greeting_tag);
        let features = choices
            .features
            .iter()
            .copied()
            .filter_map(Self::feature_tag);
        greetings.chain(features).collect()
    }

    fn has_greeting(language: TacLanguage) -> bool {
        language != TacLanguage::JavaScript
    }

    fn template_files(language: TacLanguage) -> Vec<(String, String)> {
        language.files()
    }

    fn javascript_files(language: TacLanguage, javascript_name: &str) -> Vec<(String, String)> {
        if language != TacLanguage::JavaScript {
            return Vec::new();
        }
        Self::shell_service_files(javascript_name)
    }

    fn greeting_files(language: TacLanguage) -> Vec<(String, String)> {
        let instance = language.instance();
        let label = language.label();
        Self::greeting_tag(language).map_or_else(Vec::new, |_| {
            vec![
                (
                    format!("client/components/greeting/{}/tac.html", language.slug()),
                    format!(
                        "<section class=\"greeting\">\n  <p>{{{instance}.message}}</p>\n  <button on:click=\"{instance}.greet()\">Say hello in {label}</button>\n</section>\n"
                    ),
                ),
                (
                    format!("client/components/greeting/{}/tac.css", language.slug()),
                    String::from(".greeting { margin-top: 1.5rem; }\n"),
                ),
            ]
        })
    }

    fn shell_view(languages: &[TacLanguage]) -> String {
        let brand = Self::brand(languages.contains(&TacLanguage::JavaScript));
        format!(
            "<div class=\"shell\">\n  <div class=\"brand\">\n    <strong>{brand}</strong>\n    <nav>\n      <a href=\"/\">Home</a>\n    </nav>\n  </div>\n\n  <slot />\n</div>\n"
        )
    }

    fn brand(has_shell_service: bool) -> &'static str {
        if has_shell_service {
            return Self::SHELL_BRAND;
        }
        Self::PLAIN_BRAND
    }

    fn fixed_files(javascript_name: &str, choices: &ProjectChoices) -> Vec<(String, String)> {
        let files = [
            (
                Self::MANIFEST_PATH,
                Self::manifest(javascript_name, &choices.tac),
            ),
            (
                "client/jsconfig.json",
                BaseFiles::jsconfig("client", choices.client_uses_typescript()),
            ),
            (
                "client/shared/styles/app.css",
                String::from(
                    ":root {\n  color-scheme: dark;\n}\nbody { margin: 0; font-family: \"IBM Plex Sans\", ui-sans-serif, system-ui, sans-serif; background: #0f172a; color: #e2e8f0; }\n",
                ),
            ),
            (Self::ICON_PATH, String::from(Self::ICON)),
            ("client/shared/data/.gitkeep", String::new()),
            (
                "client/components/app/shell/tac.css",
                String::from(
                    ".shell { max-width: 72rem; margin: 0 auto; padding: 2rem 1.25rem 4rem; }\n.brand { display: flex; align-items: center; justify-content: space-between; margin-bottom: 2rem; }\n.brand a { color: inherit; text-decoration: none; }\n",
                ),
            ),
            (
                "client/components/app/hero/tac.html",
                String::from(
                    "<section class=\"hero\">\n  <h1>Build your next app with Tachyon.</h1>\n  <p>File-system routes, reactive Tac pages, static export, and preview tooling are already wired in.</p>\n</section>\n",
                ),
            ),
            (
                "client/components/app/hero/tac.css",
                String::from(
                    ".hero { padding: 2rem; border-radius: 1.5rem; background: linear-gradient(135deg, #1d4ed8, #0f766e); }\n.hero h1 { margin: 0 0 0.75rem; font-size: clamp(2rem, 6vw, 4rem); }\n.hero p { margin: 0; max-width: 42rem; line-height: 1.6; }\n",
                ),
            ),
        ];
        files
            .into_iter()
            .map(|(path, text)| (String::from(path), text))
            .collect()
    }

    fn manifest(javascript_name: &str, languages: &[TacLanguage]) -> String {
        let python = if languages.contains(&TacLanguage::Python) {
            format!(",\n    \"python\": {}", Self::python_runtime())
        } else {
            String::new()
        };
        format!(
            "{{\n  \"name\": {javascript_name},\n  \"short_name\": {javascript_name},\n  \"start_url\": \"/\",\n  \"display\": \"standalone\",\n  \"icons\": [\n    {{\"src\": \"/shared/assets/icon.svg\", \"sizes\": \"any\", \"type\": \"image/svg+xml\"}},\n    {{\"src\": \"/shared/assets/icon-192.png\", \"sizes\": \"192x192\", \"type\": \"image/png\"}},\n    {{\"src\": \"/shared/assets/icon-512.png\", \"sizes\": \"512x512\", \"type\": \"image/png\"}}\n  ],\n  \"tac\": {{\n    \"styles\": [\"/shared/styles/app.css\"],\n    \"styleSources\": {{\n      \"/shared/styles/app.css\": {{\"kind\": \"foundation\"}}\n    }}{python}\n  }}\n}}\n"
        )
    }

    pub(crate) fn python_runtime() -> serde_json::Value {
        serde_json::json!({
            "runtime": RuntimeAssets::RUNTIME,
            "profile": RuntimeAssets::PROFILE,
        })
    }

    fn shell_service_files(javascript_name: &str) -> Vec<(String, String)> {
        vec![
            (
                String::from("client/services/ShellService.js"),
                format!(
                    "@Service\nexport default class ShellService {{\n  brand = 'Tachyon'\n\n  /** @type {{DocumentTitle}} */\n  @Needs\n  documentTitle\n\n  @OnStart\n  initialize() {{\n    this.documentTitle.set({javascript_name})\n  }}\n}}\n"
                ),
            ),
            (
                String::from("client/tests/ShellTestSetup.js"),
                String::from(
                    "@TestSetup\nexport default class ShellTestSetup {\n  static configure() {\n    return {}\n  }\n}\n",
                ),
            ),
            (
                String::from("client/tests/ShellServiceTest.js"),
                format!(
                    "@Test(ShellTestSetup, {{\"partition\":\"tests\"}})\nexport default class ShellServiceTest {{\n  /** @type {{DocumentTitle}} */\n  @Fake\n  documentTitle\n\n  async testInitializeNamesTheDocument() {{\n    let title = ''\n    this.documentTitle = {{ set: (value) => {{ title = value }} }}\n    this.construct(ShellService).initialize()\n    Assert.equals({javascript_name}, title)\n  }}\n}}\n",
                ),
            ),
        ]
    }
}
