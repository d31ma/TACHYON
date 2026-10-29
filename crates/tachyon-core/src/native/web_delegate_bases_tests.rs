use super::registry::NativeCompanionInput;
use super::web_delegate_bases::WebDelegateBases;
use crate::project::NativeCompanion;

struct WebDelegateFixture;

impl WebDelegateFixture {
    const SERVICE: &'static str =
        "#[Service]\n#[derive(Default)]\npub struct CounterService {\n    pub count: i64,\n}\n";
    const TAKER: &'static str = "#[Delegate]\n#[derive(Default)]\npub struct CountDelegate {\n    #[Needs]\n    counter_service: std::sync::Arc<CounterService>,\n}\n";
    const PLAIN: &'static str =
        "#[Delegate]\n#[derive(Default)]\npub struct CountDelegate {\n    step: i64,\n}\n";

    fn refused(
        delegate: (&str, &str),
        language: NativeCompanion,
        web_assembly: bool,
    ) -> Option<String> {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("fixture root: {error}"));
        for (relative, text) in [
            ("client/services/CounterService.rs", Self::SERVICE),
            delegate,
        ] {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().unwrap_or(root.path()))
                .unwrap_or_else(|error| panic!("fixture directory: {error}"));
            std::fs::write(path, text).unwrap_or_else(|error| panic!("fixture source: {error}"));
        }
        let input = NativeCompanionInput {
            snapshot_root: root.path().to_path_buf(),
            language,
            source: root.path().join("client/services/CounterService.rs"),
            service: String::from("counter"),
            class: String::from("CounterService"),
            web_assembly,
            application_provider: false,
        };
        WebDelegateBases::refuse(&[&input])
            .err()
            .as_ref()
            .map(ToString::to_string)
    }

    fn assert_kept(relative: &str, delegate: &str, language: NativeCompanion, web_assembly: bool) {
        let refused = Self::refused((relative, delegate), language, web_assembly);
        assert_eq!(refused, None, "{relative} must build");
    }
}

#[test]
fn a_rust_browser_delegate_that_takes_on_a_service_is_refused_at_its_field() {
    let text = WebDelegateFixture::refused(
        (
            "client/delegates/CountDelegate.rs",
            WebDelegateFixture::TAKER,
        ),
        NativeCompanion::Rust,
        true,
    )
    .unwrap_or_else(|| panic!("CountDelegate takes on CounterService and must be refused"));
    assert!(text.contains("TY1406"), "{text}");
    assert!(
        text.contains("CountDelegate takes on CounterService, but a Rust browser service cannot need another service"),
        "{text}"
    );
    assert!(text.contains("client/delegates/CountDelegate.rs"), "{text}");
}

#[test]
fn a_native_rust_delegate_keeps_its_base_service() {
    WebDelegateFixture::assert_kept(
        "client/delegates/CountDelegate.rs",
        WebDelegateFixture::TAKER,
        NativeCompanion::Rust,
        false,
    );
}

#[test]
fn a_rust_browser_delegate_without_a_base_is_kept() {
    WebDelegateFixture::assert_kept(
        "client/delegates/CountDelegate.rs",
        WebDelegateFixture::PLAIN,
        NativeCompanion::Rust,
        true,
    );
}

#[test]
fn only_rust_delegates_of_a_rust_browser_build_are_read() {
    WebDelegateFixture::assert_kept(
        "client/delegates/CountDelegate.rs",
        WebDelegateFixture::TAKER,
        NativeCompanion::Kotlin,
        true,
    );
    WebDelegateFixture::assert_kept(
        "client/delegates/CountDelegate.js",
        WebDelegateFixture::TAKER,
        NativeCompanion::Rust,
        true,
    );
}
