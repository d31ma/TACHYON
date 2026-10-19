use super::config::NativeApplication;
use super::host::xml_escape;
use super::linux_package_names::LinuxPackageNames;
use tachyon_contracts::NativeTarget;

pub(super) struct LinuxPackageFiles;

impl LinuxPackageFiles {
    pub(super) const MAINTAINER: &'static str = "TAC_LINUX_MAINTAINER";
    const SUMMARY_END: &'static [char] = &['.', ' '];
    const SECONDS_PER_DAY: u64 = 86_400;
    const DAYS_FROM_MARCH_OF_YEAR_ZERO_TO_1970: u64 = 719_468;
    const DAYS_PER_400_YEARS: u64 = 146_097;

    pub(super) fn desktop_entry(
        application: &NativeApplication,
        names: &LinuxPackageNames,
    ) -> String {
        let icon = application
            .largest_icon()
            .map_or_else(String::new, |_| format!("Icon={}\n", names.desktop_id));
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={name}\n\
             Comment={summary}\n\
             Exec={package}{exec_argument}\n\
             {icon}{mime_types}\
             Terminal=false\n\
             Categories=Utility;\n\
             X-Tachyon-Version={version}\n\
             X-Tachyon-Build={build}\n",
            name = Self::desktop_value(&application.name),
            summary = Self::desktop_value(&Self::summary(application)),
            package = names.package,
            exec_argument = application.links.desktop_exec_argument(),
            mime_types = application.links.desktop_mime_types(),
            version = Self::desktop_value(&application.version),
            build = application.build,
        )
    }

    pub(super) fn metainfo(
        application: &NativeApplication,
        names: &LinuxPackageNames,
        epoch: u64,
    ) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>{id}</id>
  <metadata_license>CC0-1.0</metadata_license>
  <name>{name}</name>
  <summary>{summary}</summary>
  <description>
    <p>{description}</p>
  </description>
  <launchable type="desktop-id">{id}.desktop</launchable>
  <provides>
    <binary>{package}</binary>
  </provides>
  <releases>
    <release version="{version}" date="{date}"/>
  </releases>
</component>
"#,
            id = names.desktop_id,
            name = xml_escape(&application.name),
            summary = xml_escape(&Self::summary(application)),
            description = xml_escape(&Self::description(application)),
            package = names.package,
            version = xml_escape(&application.version),
            date = Self::release_date(epoch),
        )
    }

    pub(super) fn control(
        application: &NativeApplication,
        names: &LinuxPackageNames,
        maintainer: Option<&str>,
    ) -> String {
        let floor = application.minimum_system.floor(NativeTarget::Linux);
        let maintainer = maintainer
            .and_then(|value| value.lines().next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map_or_else(|| Self::summary_name(application), String::from);
        format!(
            "Package: {package}\n\
             Version: {version}\n\
             Architecture: {architecture}\n\
             Maintainer: {maintainer}\n\
             Depends: libc6, libgtk-4-1, libwebkitgtk-6.0-4 (>= {major}.{minor})\n\
             Section: utils\n\
             Priority: optional\n\
             Description: {summary}\n \
             {description}\n",
            package = names.package,
            version = names.version,
            architecture = names.deb_architecture,
            major = floor.major(),
            minor = floor.minor(),
            summary = Self::summary(application),
            description = Self::description(application),
        )
    }

    fn summary_name(application: &NativeApplication) -> String {
        Self::desktop_value(application.name.trim())
    }

    pub(super) fn summary(application: &NativeApplication) -> String {
        let first_line = application.description.lines().next().unwrap_or_default();
        let summary = first_line.trim().trim_end_matches(Self::SUMMARY_END);
        [summary, application.name.trim()]
            .into_iter()
            .find(|text| !text.is_empty())
            .map_or_else(|| application.executable_name.clone(), String::from)
    }

    fn description(application: &NativeApplication) -> String {
        let description = application.description.split_whitespace();
        let joined = description.collect::<Vec<_>>().join(" ");
        [joined, Self::summary(application)]
            .into_iter()
            .find(|text| !text.is_empty())
            .unwrap_or_default()
    }

    fn release_date(epoch: u64) -> String {
        let days = epoch / Self::SECONDS_PER_DAY + Self::DAYS_FROM_MARCH_OF_YEAR_ZERO_TO_1970;
        let era = days / Self::DAYS_PER_400_YEARS;
        let day_of_era = days - era * Self::DAYS_PER_400_YEARS;
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
        let month = (month_from_march + 2) % 12 + 1;
        let year = year_of_era + era * 400 + u64::from(month <= 2);
        format!("{year:04}-{month:02}-{day:02}")
    }

    fn desktop_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::LinuxPackageFiles;
    use crate::native::config::NativeApplication;
    use crate::native::linux_package_names::LinuxPackageNames;

    fn application() -> NativeApplication {
        let mut application = crate::native::linux_host_tests::LinuxHostTests::application(
            crate::native::native_menu_config::NativeMenuConfig::default(),
        );
        application.name = String::from("Linux <Gate>");
        application.executable_name = String::from("LinuxGate");
        application.application_id = String::from("dev.tachyon.linux-gate");
        application.version = String::from("0.0.1");
        application.build = 7;
        application.description = String::from("Shows the gate & its routes.\nSecond line.");
        application.icons = vec![(256, String::from("/icon-256.png"))];
        application
    }

    fn names(application: &NativeApplication) -> LinuxPackageNames {
        LinuxPackageNames::of(application, "x86_64").expect("names")
    }

    #[test]
    fn the_installed_desktop_entry_runs_the_package_command_and_names_the_themed_icon() {
        let application = application();
        let entry = LinuxPackageFiles::desktop_entry(&application, &names(&application));
        assert!(entry.starts_with("[Desktop Entry]\nType=Application\nName=Linux <Gate>\n"));
        assert!(entry.contains("Comment=Shows the gate & its routes\n"));
        assert!(entry.contains("Exec=linuxgate\nIcon=dev.tachyon.linux_gate\n"));
        assert!(entry.contains("X-Tachyon-Version=0.0.1\nX-Tachyon-Build=7\n"));
        let mut plain = application.clone();
        plain.icons.clear();
        let entry = LinuxPackageFiles::desktop_entry(&plain, &names(&plain));
        assert!(!entry.contains("Icon="));
        assert!(!entry.contains("MimeType="));
    }

    #[test]
    fn the_installed_desktop_entry_claims_the_declared_link_schemes() {
        let mut application = application();
        application.links = crate::native::native_links::NativeLinks::declared(&["gate"], &[]);
        let entry = LinuxPackageFiles::desktop_entry(&application, &names(&application));
        assert!(entry.contains(
            "Exec=linuxgate %u\nIcon=dev.tachyon.linux_gate\nMimeType=x-scheme-handler/gate;\n"
        ));
    }

    #[test]
    fn the_metainfo_names_the_desktop_file_and_the_release() {
        let application = application();
        let metainfo =
            LinuxPackageFiles::metainfo(&application, &names(&application), 1_700_000_000);
        assert!(metainfo.contains("<id>dev.tachyon.linux_gate</id>"));
        assert!(metainfo.contains("<name>Linux &lt;Gate&gt;</name>"));
        assert!(metainfo.contains("<summary>Shows the gate &amp; its routes</summary>"));
        assert!(metainfo.contains("<p>Shows the gate &amp; its routes. Second line.</p>"));
        assert!(metainfo.contains(
            r#"<launchable type="desktop-id">dev.tachyon.linux_gate.desktop</launchable>"#
        ));
        assert!(metainfo.contains("<binary>linuxgate</binary>"));
        assert!(metainfo.contains(r#"<release version="0.0.1" date="2023-11-14"/>"#));
    }

    #[test]
    fn the_release_date_is_the_utc_day_of_the_epoch() {
        for (epoch, date) in [
            (0, "1970-01-01"),
            (86_399, "1970-01-01"),
            (951_782_400, "2000-02-29"),
            (951_868_800, "2000-03-01"),
            (1_700_000_000, "2023-11-14"),
            (4_107_542_399, "2100-02-28"),
        ] {
            assert_eq!(LinuxPackageFiles::release_date(epoch), date, "{epoch}");
        }
    }

    #[test]
    fn the_control_file_depends_on_the_webkitgtk_floor_and_keeps_one_maintainer_line() {
        let application = application();
        let names = names(&application);
        let control = LinuxPackageFiles::control(&application, &names, None);
        assert_eq!(
            control,
            "Package: linuxgate\nVersion: 0.0.1-7\nArchitecture: amd64\n\
             Maintainer: Linux <Gate>\n\
             Depends: libc6, libgtk-4-1, libwebkitgtk-6.0-4 (>= 2.40)\nSection: utils\n\
             Priority: optional\nDescription: Shows the gate & its routes\n \
             Shows the gate & its routes. Second line.\n"
        );
        let signed = LinuxPackageFiles::control(
            &application,
            &names,
            Some("Ada Lovelace <ada@example.com>\nPackage: other"),
        );
        assert!(
            signed.contains(
                "Architecture: amd64\nMaintainer: Ada Lovelace <ada@example.com>\nDepends:"
            )
        );
        assert!(!signed.contains("Package: other"));
    }

    #[test]
    fn the_summary_falls_back_to_the_app_name() {
        let mut application = application();
        application.description = String::from("   \n");
        assert_eq!(LinuxPackageFiles::summary(&application), "Linux <Gate>");
        let control = LinuxPackageFiles::control(&application, &names(&application), None);
        assert!(control.ends_with("Description: Linux <Gate>\n Linux <Gate>\n"));
    }
}
