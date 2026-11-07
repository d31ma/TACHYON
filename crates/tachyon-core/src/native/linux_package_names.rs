use super::config::NativeApplication;
use crate::Failure;
use crate::failure::diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LinuxPackageNames {
    pub(super) package: String,
    pub(super) desktop_id: String,
    pub(super) version: String,
    pub(super) machine: &'static str,
    pub(super) deb_architecture: &'static str,
}

impl LinuxPackageNames {
    const ARCHITECTURES: [(&'static str, &'static str); 2] =
        [("x86_64", "amd64"), ("aarch64", "arm64")];
    const VERSION_PUNCTUATION: &'static str = ".+~-";
    const SHORTEST_PACKAGE: usize = 2;

    pub(super) fn of(application: &NativeApplication, machine: &str) -> Result<Self, Failure> {
        let (machine, deb_architecture) = Self::architecture(machine)?;
        Self::check_version(&application.version)?;
        let package = application.executable_name.to_ascii_lowercase();
        Self::check_package(&package, &application.name)?;
        Ok(Self {
            package,
            desktop_id: application.application_id.replace('-', "_"),
            version: format!("{}-{}", application.version, application.build),
            machine,
            deb_architecture,
        })
    }

    pub(super) fn deb_file(&self) -> String {
        format!(
            "{}_{}_{}.deb",
            self.package, self.version, self.deb_architecture
        )
    }

    pub(super) fn app_image_file(&self, application: &NativeApplication) -> String {
        format!(
            "{}-{}-{}.AppImage",
            application.executable_name, self.version, self.machine
        )
    }

    pub(super) fn installed_folder(&self) -> String {
        format!("usr/lib/{}", self.package)
    }

    fn architecture(machine: &str) -> Result<(&'static str, &'static str), Failure> {
        Self::ARCHITECTURES
            .into_iter()
            .find(|(name, _)| *name == machine)
            .ok_or_else(|| {
                Self::refusal(format!(
                    "Linux packages are built for x86_64 and aarch64 build machines, but this machine is {machine}."
                ))
            })
    }

    fn check_version(version: &str) -> Result<(), Failure> {
        let starts_with_digit = version.starts_with(|first: char| first.is_ascii_digit());
        let every_character_allowed = version.chars().all(Self::version_character);
        let accepted = [starts_with_digit, every_character_allowed] == [true, true];
        accepted.then_some(()).ok_or_else(|| {
            Self::refusal(format!(
                "tac.application.version '{version}' cannot be a Debian package version. It must start with a digit and hold only letters, digits and . + ~ -."
            ))
        })
    }

    fn version_character(character: char) -> bool {
        [
            character.is_ascii_alphanumeric(),
            Self::VERSION_PUNCTUATION.contains(character),
        ]
        .contains(&true)
    }

    fn check_package(package: &str, name: &str) -> Result<(), Failure> {
        (package.len() >= Self::SHORTEST_PACKAGE)
            .then_some(())
            .ok_or_else(|| {
                Self::refusal(format!(
                    "The app name '{name}' gives the Linux package name '{package}', but a package name needs at least two letters or digits."
                ))
            })
    }

    fn refusal(message: String) -> Failure {
        Failure::one(diagnostic(
            1601,
            message,
            Some(String::from(
                "Fix client/manifest.json, or build Linux without --release.",
            )),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::LinuxPackageNames;
    use crate::native::config::NativeApplication;

    fn application(version: &str) -> NativeApplication {
        let mut application = crate::native::linux_host_tests::LinuxHostTests::application(
            crate::native::native_menu_config::NativeMenuConfig::default(),
        );
        application.name = String::from("Linux Gate");
        application.executable_name = String::from("LinuxGate");
        application.application_id = String::from("dev.tachyon.linux-gate");
        application.version = String::from(version);
        application.build = 7;
        application
    }

    #[test]
    fn the_names_follow_the_manifest_and_the_build_machine() {
        let application = application("2.1.0+beta1");
        let names = LinuxPackageNames::of(&application, "x86_64").expect("names");
        assert_eq!(names.package, "linuxgate");
        assert_eq!(names.desktop_id, "dev.tachyon.linux_gate");
        assert_eq!(names.version, "2.1.0+beta1-7");
        assert_eq!(names.deb_file(), "linuxgate_2.1.0+beta1-7_amd64.deb");
        assert_eq!(
            names.app_image_file(&application),
            "LinuxGate-2.1.0+beta1-7-x86_64.AppImage"
        );
        assert_eq!(names.installed_folder(), "usr/lib/linuxgate");
        let arm = LinuxPackageNames::of(&application, "aarch64").expect("arm");
        assert_eq!(arm.deb_file(), "linuxgate_2.1.0+beta1-7_arm64.deb");
    }

    #[test]
    fn a_version_debian_cannot_hold_is_refused() {
        for version in ["v1.0", "1.0 beta", "1.0/2", ""] {
            let text = LinuxPackageNames::of(&application(version), "x86_64")
                .expect_err(version)
                .to_string();
            assert!(text.contains("TY1601"), "{text}");
            assert!(text.contains("Debian package version"), "{text}");
        }
    }

    #[test]
    fn an_unknown_machine_or_a_one_letter_name_is_refused() {
        let text = LinuxPackageNames::of(&application("1.0.0"), "riscv64")
            .expect_err("machine")
            .to_string();
        assert!(text.contains("this machine is riscv64"), "{text}");
        let mut short = application("1.0.0");
        short.executable_name = String::from("A");
        let text = LinuxPackageNames::of(&short, "x86_64")
            .expect_err("short")
            .to_string();
        assert!(text.contains("at least two letters or digits"), "{text}");
    }
}
