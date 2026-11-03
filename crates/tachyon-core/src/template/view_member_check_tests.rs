use super::{ClientViewRenderer, ComponentRegistry, Scope, ServiceRegistry, TemplateFrontend};
use crate::Failure;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

struct MemberViews;

impl MemberViews {
    const PANEL: &'static str = "client/components/settings/panel/tac.html";

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, source).expect("source");
    }

    fn project(panel: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, source) in [
            (
                "client/services/SettingsService.go",
                "// @Service\ntype SettingsService struct {\n\tOAuthNotice string\n\tTitle string\n}\n\nfunc (service *SettingsService) SaveOAuth() {}\n",
            ),
            (
                "client/services/DrawerService.rs",
                "#[Service]\n#[derive(Default)]\nstruct DrawerService {}\n\nimpl DrawerService {\n    pub fn open_drawer(&mut self) {}\n\n    fn close_drawer(&mut self) {}\n}\n",
            ),
            (
                "client/services/ShellService.js",
                "@Service\nexport default class ShellService { oAuthNotice = 'shell'; }",
            ),
            (
                "client/components/settings/frame/tac.html",
                "<section><h2>{settings.title}</h2><settings-panel /></section>",
            ),
            (Self::PANEL, panel),
        ] {
            Self::write(root.path(), relative, source);
        }
        root
    }

    fn render(panel: &str, page: &str) -> Result<(), Failure> {
        let root = Self::project(panel);
        let components = ComponentRegistry::discover(root.path()).expect("components");
        let services = ServiceRegistry::discover(root.path()).expect("services");
        let program = TemplateFrontend::compile(page, "client/pages/tac.html", &components.names())
            .expect("page");
        ClientViewRenderer::new(&components, &services, &BTreeSet::new())
            .render(&program, "index.html", &Scope::new(), "/")
            .map(|_| ())
    }

    fn refused(panel: &str) -> String {
        Self::render(panel, "<settings-panel />")
            .expect_err("a near-miss member is refused")
            .to_string()
    }
}

#[test]
fn a_go_member_written_in_the_wrong_case_suggests_the_exposed_name() {
    for panel in [
        "<p>{settings.oauthNotice}</p>",
        "<p :title=\"settings.oauthNotice\">Notice</p>",
        "<input aria-label=\"Notice\" on:input=\"settings.oauthNotice = $event.target.value\" />",
    ] {
        let text = MemberViews::refused(panel);
        assert!(text.contains("TY1311"), "{panel}: {text}");
        assert!(
            text.contains(
                "The view reads settings.oauthNotice, but oauthNotice is not a member of SettingsService; did you mean oAuthNotice?"
            ),
            "{panel}: {text}"
        );
        assert!(text.contains("OAuthNotice is oAuthNotice"), "{text}");
        assert!(text.contains(MemberViews::PANEL), "{text}");
    }
    let text = MemberViews::refused("<button on:click=\"settings.saveOauth()\">Save</button>");
    assert!(text.contains("did you mean saveOAuth?"), "{text}");
}

#[test]
fn exact_go_names_script_services_and_unrelated_names_are_not_refused() {
    for panel in [
        "<p>{settings.oAuthNotice}</p><button on:click=\"settings.saveOAuth()\">Save</button>",
        "<p>{shell.oauthNotice}</p>",
        "<p>{settings.unrelated}</p>",
    ] {
        let rendered = MemberViews::render(panel, "<settings-panel />");
        assert!(rendered.is_ok(), "{panel}: {rendered:?}");
    }
}

#[test]
fn a_nested_component_is_reported_once_at_its_own_view() {
    let failure = MemberViews::render("<p>{settings.oauthNotice}</p>", "<settings-frame />")
        .expect_err("refused");
    assert_eq!(failure.diagnostics().len(), 1, "{failure}");
    let text = failure.to_string();
    assert!(text.contains(MemberViews::PANEL), "{text}");
    assert!(!text.contains("settings/frame"), "{text}");
}

#[test]
fn a_view_that_reads_a_private_rust_method_is_told_to_make_it_pub() {
    let text = MemberViews::refused("<button on:click=\"drawer.closeDrawer()\">Close</button>");
    assert!(text.contains("TY1311"), "{text}");
    assert!(
        text.contains(
            "The view reads drawer.closeDrawer, but close_drawer is a private method of DrawerService."
        ),
        "{text}"
    );
    assert!(
        text.contains("Write `pub fn close_drawer` in DrawerService."),
        "{text}"
    );
    let rendered = MemberViews::render(
        "<button on:click=\"drawer.openDrawer()\">Open</button>",
        "<settings-panel />",
    );
    assert!(rendered.is_ok(), "{rendered:?}");
}
