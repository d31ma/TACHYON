use crate::Failure;
use crate::failure::diagnostic;
use crate::native::SiteSettings;
use crate::routing::MountPath;

pub(super) struct MountedApp;

impl MountedApp {
    const PUSH_GRANT: &'static str = "push";
    const PUSH: &'static str = "grants push. Web Push has one worker for the whole site, so grant push in the shell's manifest (ADR 0138).";
    const LINKS: &'static str = "declares tac.application.links. Deep links open any page of the site, so declare them in the shell's manifest (ADR 0138).";
    const SITEMAP: &'static str = "declares tac.sitemap. The site has one site map, so declare it in the shell's manifest (ADR 0138).";
    const HELP: &'static str =
        "Move the setting into the manifest of the site's shell, the app at /.";

    pub(super) fn check(site: &SiteSettings, mount: &MountPath) -> Result<(), Failure> {
        let problems = Self::problems(site);
        Self::refuse(problems.first().copied(), mount)
    }

    fn problems(site: &SiteSettings) -> Vec<&'static str> {
        let found = [
            Self::grants_push(site),
            !site.link_hosts.is_empty(),
            !site.sitemap.is_empty(),
        ];
        Self::flagged(found)
    }

    fn grants_push(site: &SiteSettings) -> bool {
        site.browser_apis
            .iter()
            .any(|grant| grant == Self::PUSH_GRANT)
    }

    fn flagged(found: [bool; 3]) -> Vec<&'static str> {
        found
            .into_iter()
            .zip([Self::PUSH, Self::LINKS, Self::SITEMAP])
            .filter(|(flag, _)| *flag)
            .map(|(_, message)| message)
            .collect()
    }

    fn refuse(problem: Option<&str>, mount: &MountPath) -> Result<(), Failure> {
        match (mount.is_root(), problem) {
            (false, Some(message)) => Err(Self::failure(mount, message)),
            _ => Ok(()),
        }
    }

    fn failure(mount: &MountPath, message: &str) -> Failure {
        Failure::one(diagnostic(
            1601,
            format!("The page app at tac.mount {} {message}", mount.prefix()),
            Some(String::from(Self::HELP)),
            None,
        ))
    }
}
