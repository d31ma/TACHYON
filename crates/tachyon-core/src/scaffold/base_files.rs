use super::{AppName, AppTarget, ProjectChoices, TacLanguage};

pub(crate) struct BaseFiles;

impl BaseFiles {
    const GITIGNORE: &'static str = "node_modules\ndist\n.env\n.DS_Store\n.tachyon/\n";
    const TYPESCRIPT: &'static str = "7.0.2";
    const FIELDS_FILLED_LATER: &'static str = "    \"strictPropertyInitialization\": false,\n";

    pub(crate) fn for_choices(app_name: &str, choices: &ProjectChoices) -> Vec<(String, String)> {
        vec![
            (String::from(".gitignore"), String::from(Self::GITIGNORE)),
            (
                String::from("tachyon-env.d.ts"),
                String::from(include_str!("../../../../types/tachyon-env.d.ts")),
            ),
            (
                String::from("tachyon-behaviors.d.ts"),
                String::from(include_str!("../../../../types/tachyon-behaviors.d.ts")),
            ),
            (String::from("README.md"), Self::readme(app_name)),
            (
                String::from("package.json"),
                Self::package_json(app_name, choices),
            ),
        ]
    }

    pub(crate) fn jsconfig(realm: &str, typescript_only: bool) -> String {
        let fields = Self::field_check(typescript_only);
        format!(
            "{{\n  \"compilerOptions\": {{\n    \"strict\": true,\n{fields}    \"allowJs\": true,\n    \"checkJs\": true,\n    \"experimentalDecorators\": true,\n    \"target\": \"ESNext\",\n    \"module\": \"NodeNext\",\n    \"moduleResolution\": \"NodeNext\",\n    \"allowImportingTsExtensions\": true,\n    \"noEmit\": true,\n    \"paths\": {{\n      \"/*\": [\"./*\"]\n    }},\n    \"types\": []\n  }},\n  \"include\": [\n    \"../tachyon-env.d.ts\",\n    \"../.tachyon/types/{realm}\",\n    \"**/*.js\",\n    \"**/*.ts\"\n  ]\n}}\n"
        )
    }

    fn field_check(typescript_only: bool) -> &'static str {
        if typescript_only {
            return "";
        }
        Self::FIELDS_FILLED_LATER
    }

    fn package_json(app_name: &str, choices: &ProjectChoices) -> String {
        let package_name = AppName::package(app_name);
        let target_scripts: String = choices
            .native_targets()
            .into_iter()
            .map(AppTarget::bundle_script)
            .collect();
        let development = Self::development_dependencies(choices);
        format!(
            "{{\n  \"name\": \"{package_name}\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"scripts\": {{\n{target_scripts}    \"serve\": \"ty serve\",\n    \"bundle\": \"ty bundle\",\n    \"preview\": \"ty preview --watch\",\n    \"test\": \"ty test --coverage\"\n  }}{development}\n}}\n"
        )
    }

    fn development_dependencies(choices: &ProjectChoices) -> String {
        if !choices.tac.contains(&TacLanguage::TypeScript) {
            return String::new();
        }
        format!(
            ",\n  \"devDependencies\": {{\n    \"typescript\": \"{}\"\n  }}",
            Self::TYPESCRIPT
        )
    }

    fn readme(app_name: &str) -> String {
        format!(
            "# {app_name}\n\n\
             ## Install the CLI\n\n\
             On macOS or Linux:\n\n\
             ```sh\ncurl -fsSL https://github.com/d31ma/Tachyon/releases/latest/download/install.sh | sh\n```\n\n\
             On Windows, in PowerShell:\n\n\
             ```powershell\nirm https://github.com/d31ma/Tachyon/releases/latest/download/install.ps1 | iex\n```\n\n\
             The installer checks the release checksum and installs only the `ty` binary.\n\n\
             ## Commands\n\n\
             | Command | What it does |\n|---|---|\n\
             | `ty bundle` | Builds the client and native artifacts into `dist/`. |\n\
             | `ty preview` | Previews the built bundle. |\n\
             | `ty serve` | Runs the development or production server. |\n\
             | `ty test --coverage` | Runs every test unit and checks that each public method has a test. |\n\
             | `ty add language client ts` | Adds a language, a target (`ty add target ios`) or an example (`ty add feature sockets`). |\n\n\
             `ty serve` finds out whether the app has `client/`, `server/`, or both, and serves the frontend, the backend, or both.\n\n\
             ## Application configuration\n\n\
             - `client/manifest.json` is the web app manifest. Its `tac` member is Tac's public configuration, and everything in it is sent to the browser.\n\
             - `server/application.json` belongs to Yon only. Its members are Yon's configuration without secrets, read as `yon.*` paths.\n\
             - The server's host and port come from the CLI arguments first, then from `server/application-<profile>.json` (chosen with `YON_PROFILE`), then from `server/application.json`.\n\
             - Datasource and Redis settings configure adapters that the application owns. They do not open connections by themselves.\n\
             - Put secrets only in `.env` at the project root, for example `YON_DATASOURCE_PASSWORD=...`. Each underscore becomes a nested lowercase key (`yon.datasource.password`). Never commit credentials.\n\
             - Yon handlers can read their private `YON_APPLICATION_JSON` snapshot.\n\
             - Only `client/manifest.json` reaches Tac builds, and builds never open `.env`. Settings from the parent environment cannot override application values.\n\
             - Restart the server after you change its configuration or secrets.\n"
        )
    }
}
