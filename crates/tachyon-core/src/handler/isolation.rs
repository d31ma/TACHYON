mod environment_names;
mod firecracker_isolation;
mod warm_settings;

pub use firecracker_isolation::FirecrackerIsolation;
pub use warm_settings::WarmSettings;

use super::readiness::YonLanguage;
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::HandlerLanguage;
use environment_names::EnvironmentNames;
use std::ffi::{OsStr, OsString};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum YonIsolationPolicy {
    Warm(WarmSettings),
    Process,
    Firecracker(FirecrackerIsolation),
}

impl Default for YonIsolationPolicy {
    fn default() -> Self {
        Self::Warm(WarmSettings::default())
    }
}

impl YonIsolationPolicy {
    const SOURCE_HANDLER_LANGUAGES: [HandlerLanguage; 2] =
        [HandlerLanguage::JavaScript, HandlerLanguage::Python];
    const SOURCE_YON_LANGUAGES: [YonLanguage; 2] = [YonLanguage::JavaScript, YonLanguage::Python];
    const MODES: [&'static str; 3] = ["warm", "process", "firecracker"];

    pub fn from_environment() -> Result<Self, Failure> {
        Self::from_lookup(|name| std::env::var_os(name))
    }

    pub(crate) fn uses_direct_handler_protocol(&self) -> bool {
        self.firecracker_policy().is_none()
    }

    pub(crate) fn firecracker(&self) -> Option<&FirecrackerIsolation> {
        self.firecracker_policy()
    }

    pub(crate) fn warm(&self) -> Option<&WarmSettings> {
        self.warm_settings()
    }

    pub(super) fn from_lookup<F>(lookup: F) -> Result<Self, Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let mode = Self::optional_unicode(&lookup, EnvironmentNames::ISOLATION)?
            .unwrap_or_else(|| String::from("warm"));
        Self::check_known_mode(&mode)?;
        Self::check_values_belong_to(&mode, &lookup)?;
        Self::from_mode(&mode, &lookup)
    }

    pub(crate) fn apply_backend_environment(&self, command: &mut tokio::process::Command) {
        command.envs(self.backend_environment());
    }

    pub(crate) fn validate_backend_language(
        &self,
        language: HandlerLanguage,
    ) -> Result<(), Failure> {
        let is_source_language = Self::SOURCE_HANDLER_LANGUAGES.contains(&language);
        self.check_source_language(is_source_language, language.name())
    }

    pub(crate) fn validate_backend_yon_language(
        &self,
        language: YonLanguage,
    ) -> Result<(), Failure> {
        let is_source_language = Self::SOURCE_YON_LANGUAGES.contains(&language);
        self.check_source_language(is_source_language, language.family())
    }

    fn parse_number<T>(
        value: Option<String>,
        name: &str,
        default: T,
        minimum: T,
        maximum: T,
    ) -> Result<T, Failure>
    where
        T: Copy + Ord + std::str::FromStr,
    {
        value.map_or(Ok(default), |value| {
            Self::parse_in_range(&value, name, minimum, maximum)
        })
    }

    fn parse_in_range<T>(value: &str, name: &str, minimum: T, maximum: T) -> Result<T, Failure>
    where
        T: Copy + Ord + std::str::FromStr,
    {
        let parsed = value.parse::<T>().map_err(|_| {
            Self::configuration_failure(format!("{name} must be a base-10 integer."))
        })?;
        let in_range = (minimum..=maximum).contains(&parsed);
        if !in_range {
            return Err(Self::configuration_failure(format!(
                "{name} is outside its supported range."
            )));
        }
        Ok(parsed)
    }

    fn firecracker_policy(&self) -> Option<&FirecrackerIsolation> {
        match self {
            Self::Firecracker(policy) => Some(policy),
            Self::Warm(_) | Self::Process => None,
        }
    }

    fn warm_settings(&self) -> Option<&WarmSettings> {
        match self {
            Self::Warm(settings) => Some(settings),
            Self::Process | Self::Firecracker(_) => None,
        }
    }

    fn check_known_mode(mode: &str) -> Result<(), Failure> {
        if !Self::MODES.contains(&mode) {
            return Err(Self::configuration_failure(format!(
                "{} must be 'warm', 'process' or 'firecracker', not '{mode}'.",
                EnvironmentNames::ISOLATION
            )));
        }
        Ok(())
    }

    fn check_values_belong_to<F>(mode: &str, lookup: &F) -> Result<(), Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let foreign = EnvironmentNames::foreign_to(mode);
        let configured: Vec<&str> = foreign
            .into_iter()
            .filter(|name| lookup(name).is_some())
            .collect();
        Self::refuse_foreign_values(mode, &configured)
    }

    fn refuse_foreign_values(mode: &str, configured: &[&str]) -> Result<(), Failure> {
        if !configured.is_empty() {
            return Err(Self::configuration_failure(format!(
                "{} cannot be used with {}={mode}.",
                configured.join(", "),
                EnvironmentNames::ISOLATION
            )));
        }
        Ok(())
    }

    fn from_mode<F>(mode: &str, lookup: &F) -> Result<Self, Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        match mode {
            "firecracker" => FirecrackerIsolation::from_lookup(lookup).map(Self::Firecracker),
            "process" => Ok(Self::Process),
            _ => WarmSettings::from_lookup(lookup).map(Self::Warm),
        }
    }

    fn backend_environment(&self) -> Option<(&'static str, &'static OsStr)> {
        self.firecracker_policy()
            .map(|_| (EnvironmentNames::ISOLATION, OsStr::new("firecracker")))
    }

    fn check_source_language(
        &self,
        is_source_language: bool,
        language_name: &str,
    ) -> Result<(), Failure> {
        let refused = [self.firecracker_policy().is_some(), !is_source_language];
        if refused == [true, true] {
            return Err(Self::unsupported_language_failure(language_name));
        }
        Ok(())
    }

    fn optional_unicode<F>(lookup: &F, name: &str) -> Result<Option<String>, Failure>
    where
        F: Fn(&str) -> Option<OsString>,
    {
        lookup(name)
            .map(|value| Self::unicode(value, name))
            .transpose()
    }

    fn unicode(value: OsString, name: &str) -> Result<String, Failure> {
        value
            .into_string()
            .map_err(|_| Self::configuration_failure(format!("{name} must contain valid Unicode.")))
    }

    fn configuration_failure(message: impl Into<String>) -> Failure {
        Failure::one(diagnostic(
            2010,
            message,
            Some(String::from(
                "Set a complete environment-only Yon isolation policy before starting Tachyon.",
            )),
            None,
        ))
    }

    fn unsupported_language_failure(language_name: &str) -> Failure {
        Failure::one(diagnostic(
            2010,
            format!("Firecracker isolation cannot invoke a prepared {language_name} handler."),
            Some(String::from(
                "The current Firecracker driver contract transfers only project-contained \
                 JavaScript and Python source. Use warm or process isolation for TypeScript, Java, Groovy, \
                 PHP, Kotlin, C#, Rust, Ruby, Lua, or Go until an artifact-transfer contract is implemented.",
            )),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{WarmSettings, YonIsolationPolicy};
    use crate::handler::{HandlerLanguage, YonLanguage};
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::time::Duration;

    fn parse(values: &[(&str, &str)]) -> Result<YonIsolationPolicy, crate::Failure> {
        let values = values
            .iter()
            .map(|(name, value)| (String::from(*name), OsString::from(*value)))
            .collect::<BTreeMap<_, _>>();
        YonIsolationPolicy::from_lookup(|name| values.get(name).cloned())
    }

    fn is_refused(values: &[(&str, &str)], reason: &str) -> bool {
        parse(values)
            .expect_err(reason)
            .to_string()
            .contains("TY2010")
    }

    #[cfg(unix)]
    fn write_driver(driver: &std::path::Path, script: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(driver, script).expect("driver");
        fs::set_permissions(driver, fs::Permissions::from_mode(0o700)).expect("permissions");
    }

    #[test]
    fn warm_is_the_environment_default() {
        assert_eq!(
            parse(&[]).expect("default policy"),
            YonIsolationPolicy::Warm(WarmSettings::default())
        );
        assert_eq!(
            parse(&[("YON_ISOLATION", "warm")]).expect("warm policy"),
            YonIsolationPolicy::default()
        );
        assert_eq!(
            parse(&[("YON_ISOLATION", "process")]).expect("process policy"),
            YonIsolationPolicy::Process
        );
        assert!(YonIsolationPolicy::default().warm().is_some());
        assert!(YonIsolationPolicy::Process.warm().is_none());
    }

    #[test]
    fn warm_settings_are_bounded() {
        let policy = parse(&[
            ("YON_WARM_MAX_REQUESTS", "5"),
            ("YON_WARM_IDLE_SECONDS", "2"),
        ])
        .expect("warm settings");
        let settings = policy.warm().copied().expect("warm mode");
        assert_eq!(settings.max_requests, 5);
        assert_eq!(settings.idle_limit, std::time::Duration::from_secs(2));
        let defaults = WarmSettings::default();
        assert_eq!(defaults.max_requests, 1_000);
        assert_eq!(defaults.idle_limit, std::time::Duration::from_mins(1));
        for (name, value) in [
            ("YON_WARM_MAX_REQUESTS", "0"),
            ("YON_WARM_MAX_REQUESTS", "1000001"),
            ("YON_WARM_MAX_REQUESTS", "many"),
            ("YON_WARM_IDLE_SECONDS", "0"),
            ("YON_WARM_IDLE_SECONDS", "3601"),
        ] {
            assert!(is_refused(&[(name, value)], "bounded warm setting"));
        }
    }

    #[test]
    fn warm_settings_belong_to_warm_mode_only() {
        for mode in ["process", "firecracker"] {
            for name in ["YON_WARM_MAX_REQUESTS", "YON_WARM_IDLE_SECONDS"] {
                let rendered = parse(&[("YON_ISOLATION", mode), (name, "5")])
                    .expect_err("warm value outside warm mode")
                    .to_string();
                assert!(rendered.contains("TY2010"), "{rendered}");
                assert!(rendered.contains(name), "{rendered}");
            }
        }
        assert!(is_refused(
            &[("YON_FIRECRACKER_POOL", "tenant_a")],
            "firecracker value in the default warm mode"
        ));
    }

    #[test]
    fn invalid_or_partial_environment_policy_fails_closed() {
        for values in [
            vec![("YON_ISOLATION", "container")],
            vec![("YON_ISOLATION", "firecracker")],
            vec![("YON_FIRECRACKER_DRIVER", "/tmp/driver")],
            vec![
                ("YON_ISOLATION", "process"),
                ("YON_FIRECRACKER_POOL", "configured-but-disabled"),
            ],
            vec![("YON_ISOLATION", "warm"), ("YON_FIRECRACKER_VCPUS", "2")],
            vec![
                ("YON_ISOLATION", "firecracker"),
                ("YON_FIRECRACKER_DRIVER", "relative/driver"),
            ],
        ] {
            assert!(is_refused(&values, "invalid policy"));
        }
        assert!(
            YonIsolationPolicy::configuration_failure("invalid")
                .to_string()
                .contains("TY2010")
        );
    }

    #[test]
    fn firecracker_support_is_limited_to_source_languages() {
        for local in [YonIsolationPolicy::Process, YonIsolationPolicy::default()] {
            assert!(
                local
                    .validate_backend_language(HandlerLanguage::Direct)
                    .is_ok()
            );
        }

        let firecracker = YonIsolationPolicy::Firecracker(super::FirecrackerIsolation {
            driver: std::path::PathBuf::from("driver"),
            pool: String::from("default"),
            vcpus: 1,
            memory_mib: 256,
        });
        for language in [HandlerLanguage::JavaScript, HandlerLanguage::Python] {
            assert!(firecracker.validate_backend_language(language).is_ok());
            assert!(matches!(language.adapter(), "javascript.v1" | "python.v1"));
        }
        for language in [
            HandlerLanguage::Ruby,
            HandlerLanguage::TypeScript,
            HandlerLanguage::Direct,
        ] {
            let failure = firecracker
                .validate_backend_language(language)
                .expect_err("prepared artifact must be refused");
            assert!(failure.to_string().contains("TY2010"), "{failure}");
        }
        for language in [YonLanguage::JavaScript, YonLanguage::Python] {
            assert!(firecracker.validate_backend_yon_language(language).is_ok());
        }
        for language in [
            YonLanguage::TypeScript,
            YonLanguage::Java,
            YonLanguage::Groovy,
            YonLanguage::Php,
            YonLanguage::Kotlin,
            YonLanguage::CSharp,
            YonLanguage::Rust,
            YonLanguage::Go,
            YonLanguage::Ruby,
            YonLanguage::Lua,
            YonLanguage::Cpp,
        ] {
            let failure = firecracker
                .validate_backend_yon_language(language)
                .expect_err("unsupported deployment language must fail readiness");
            assert!(failure.to_string().contains("TY2010"), "{failure}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_refuses_a_complete_firecracker_policy_at_startup() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let driver = directory.path().join("firecracker-driver.exe");
        std::fs::write(&driver, "driver").expect("driver");
        let path = driver.to_string_lossy();
        let refused = parse(&[
            ("YON_ISOLATION", "firecracker"),
            ("YON_FIRECRACKER_DRIVER", &path),
        ])
        .expect_err("Windows has no Firecracker host");
        let rendered = refused.to_string();
        assert!(
            rendered.contains("TY2010") && rendered.contains("refused on Windows"),
            "{rendered}"
        );
        assert_eq!(
            parse(&[("YON_ISOLATION", "process")]).expect("process policy"),
            YonIsolationPolicy::Process
        );
    }

    #[cfg(unix)]
    #[test]
    fn firecracker_policy_is_bounded_and_deny_by_default() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("temporary directory");
        let driver = directory.path().join("driver");
        write_driver(&driver, "#!/bin/sh\nexit 0\n");
        let path = driver.to_string_lossy();
        let policy = parse(&[
            ("YON_ISOLATION", "firecracker"),
            ("YON_FIRECRACKER_DRIVER", &path),
            ("YON_FIRECRACKER_POOL", "tenant_a"),
            ("YON_FIRECRACKER_VCPUS", "2"),
            ("YON_FIRECRACKER_MEMORY_MIB", "512"),
        ])
        .expect("firecracker policy");
        assert!(policy.firecracker().is_some());

        for (name, value) in [
            ("YON_FIRECRACKER_POOL", "tenant/a"),
            ("YON_FIRECRACKER_VCPUS", "0"),
            ("YON_FIRECRACKER_VCPUS", "33"),
            ("YON_FIRECRACKER_MEMORY_MIB", "127"),
            ("YON_FIRECRACKER_MEMORY_MIB", "32769"),
            ("YON_FIRECRACKER_EGRESS", "allow"),
        ] {
            let values = [
                ("YON_ISOLATION", "firecracker"),
                ("YON_FIRECRACKER_DRIVER", path.as_ref()),
                (name, value),
            ];
            assert!(is_refused(&values, "bounded policy"));
        }

        fs::set_permissions(&driver, fs::Permissions::from_mode(0o722))
            .expect("unsafe permissions");
        let values = [
            ("YON_ISOLATION", "firecracker"),
            ("YON_FIRECRACKER_DRIVER", path.as_ref()),
        ];
        assert!(is_refused(&values, "writable driver"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn firecracker_driver_receives_framed_protocol_and_bounded_policy() {
        use crate::{
            HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
        };
        use tachyon_contracts::{HandlerRequest, HttpMethod};

        let directory = tempfile::tempdir().expect("temporary directory");
        let workers = directory.path().join("server/workers");
        fs::create_dir_all(&workers).expect("worker directory");
        let handler = workers.join("example.py");
        fs::write(&handler, "class Handler:\n    pass\n").expect("source");
        let source = HandlerSource::discover(
            directory.path(),
            std::path::Path::new("server/workers/example.py"),
        )
        .expect("handler source");
        let request = HandlerRequest::route(
            String::from("firecracker_test"),
            String::from("/example"),
            HttpMethod::Get,
        );
        let request_bytes =
            crate::handler::frame::HandlerFrame::request(&request).expect("request frame");
        let driver = directory.path().join("firecracker-driver");
        write_driver(&driver, &framed_driver_script(request_bytes.len()));
        let path = driver.to_string_lossy();
        let isolation = parse(&[
            ("YON_ISOLATION", "firecracker"),
            ("YON_FIRECRACKER_DRIVER", &path),
            ("YON_FIRECRACKER_POOL", "tenant_a"),
            ("YON_FIRECRACKER_VCPUS", "2"),
            ("YON_FIRECRACKER_MEMORY_MIB", "512"),
        ])
        .expect("isolation policy");
        let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
            isolation,
            default_timeout: Duration::from_secs(2),
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor");
        let received = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect("driver response");
        assert_eq!(received.status, 200);
        assert_eq!(received.body.expect("body").data, "isolated");
    }

    #[cfg(unix)]
    fn framed_driver_script(request_length: usize) -> String {
        use std::fmt::Write as _;
        use tachyon_contracts::{HandlerBody, HandlerBodyEncoding};

        let body = HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: String::from("isolated"),
        };
        let response = tachyon_contracts::HandlerResponse::success(
            String::from("firecracker_test"),
            200,
            tachyon_contracts::HandlerHeaders::new(),
            body,
        );
        let response_bytes = serde_json::to_vec(&response).expect("response JSON");
        let length = u32::try_from(response_bytes.len()).expect("bounded response");
        let mut prefix = String::new();
        for byte in length.to_be_bytes() {
            write!(prefix, "\\{byte:03o}").expect("write to string");
        }
        format!(
            "#!/bin/sh\n\
             case \"$*\" in\n\
               *\"--protocol handler-v1\"*\"--pool tenant_a\"*\"--vcpus 2\"*\"--memory-mib 512\"*\"--egress deny\"*\"--source server/workers/example.py\"*\"--adapter python.v1\"*) ;;\n\
               *) echo 'invalid driver arguments' >&2; exit 9 ;;\n\
             esac\n\
             [ \"$YON_ISOLATION\" = firecracker ] || exit 10\n\
             dd bs=1 count={} of=/dev/null 2>/dev/null\n\
             printf '{prefix}'\n\
             printf '%s' '{}'\n",
            request_length,
            String::from_utf8(response_bytes).expect("UTF-8 response")
        )
    }

    #[cfg(unix)]
    fn write_gateway_caller(root: &std::path::Path) {
        for (relative, text) in [
            (
                "server/remotes/ReportClient.py",
                "@http_client\nclass ReportClient(Protocol):\n    @gateway({\"command\":[\"python3\",\"server/helpers/report.py\"]})\n    def summarize(self, mode): ...\n",
            ),
            (
                "server/controllers/ReportController.py",
                "@request_route(\"/report\")\n@public\nclass ReportController:\n    report_client: Annotated[ReportClient, needs]\n\n    @get_route\n    def get_report(self, request):\n        return self.report_client.summarize(1)\n",
            ),
            ("server/helpers/report.py", "print(1)\n"),
        ] {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            fs::write(path, text).expect("file");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn firecracker_refuses_a_gateway_caller_at_run_time_without_starting_the_driver() {
        use crate::{
            HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
        };
        use tachyon_contracts::{HandlerRequest, HttpMethod};

        let directory = tempfile::tempdir().expect("temporary directory");
        write_gateway_caller(directory.path());
        let source =
            HandlerSource::discover(directory.path(), "server/controllers/ReportController.py")
                .expect("gateway caller");
        let marker = directory.path().join("driver-invoked");
        let driver = directory.path().join("firecracker-driver");
        let script = format!(
            "#!/bin/sh\nprintf invoked > '{}'\nexit 99\n",
            marker.display()
        );
        write_driver(&driver, &script);
        let path = driver.to_string_lossy();
        let isolation = parse(&[
            ("YON_ISOLATION", "firecracker"),
            ("YON_FIRECRACKER_DRIVER", &path),
        ])
        .expect("isolation policy");
        let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
            isolation,
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor");
        let request = HandlerRequest::route("gateway_firecracker", "/report", HttpMethod::Get);
        let refused = supervisor
            .invoke(&source, &request, &HandlerCancellation::default())
            .await
            .expect_err("gateways need the process backend");
        let rendered = refused.to_string();
        assert!(
            rendered.contains("TY2010") && rendered.contains("Gateways require"),
            "{rendered}"
        );
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn firecracker_refuses_prepared_sources_without_starting_the_driver() {
        use crate::{
            HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions,
        };
        use tachyon_contracts::{HandlerRequest, HttpMethod};

        let directory = tempfile::tempdir().expect("temporary directory");
        let marker = directory.path().join("driver-invoked");
        let driver = directory.path().join("firecracker-driver");
        let script = format!(
            "#!/bin/sh\nprintf invoked > '{}'\nexit 99\n",
            marker.display()
        );
        write_driver(&driver, &script);
        let path = driver.to_string_lossy();
        let isolation = parse(&[
            ("YON_ISOLATION", "firecracker"),
            ("YON_FIRECRACKER_DRIVER", &path),
        ])
        .expect("isolation policy");
        let supervisor = HandlerSupervisor::new(HandlerSupervisorOptions {
            isolation,
            ..HandlerSupervisorOptions::default()
        })
        .expect("supervisor");

        let cases = [
            ("server/workers/typescript.ts", "export class Handler {}\n"),
            ("server/workers/php.php", "<?php\nclass Handler {}\n"),
        ];
        for (relative, contents) in cases {
            let source_path = directory.path().join(relative);
            fs::create_dir_all(source_path.parent().expect("route parent")).expect("route");
            fs::write(&source_path, contents).expect("source");
            let source = HandlerSource::discover(directory.path(), relative).expect("discovery");
            let request = HandlerRequest::route(
                format!("firecracker_refusal_{}", source.language().name()),
                "/unsupported",
                HttpMethod::Get,
            );
            let failure = supervisor
                .invoke(&source, &request, &HandlerCancellation::default())
                .await
                .expect_err("prepared source must be refused");
            assert!(failure.to_string().contains("TY2010"), "{failure}");
            assert!(
                !marker.exists(),
                "Firecracker driver was invoked for {relative}"
            );
        }
    }
}
