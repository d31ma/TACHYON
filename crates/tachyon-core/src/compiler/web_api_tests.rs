#![allow(clippy::expect_used)]

use super::component_script::ComponentScript;
use std::fmt::Write as _;
use std::path::Path;

#[test]
fn a_javascript_service_needs_the_clock_and_secure_random() {
    let source = "@Service\nexport default class StampService {\n  @Needs\n  clock;\n  @Needs\n  secureRandom;\n\n  stamp() {\n    return `${this.clock.now()}-${this.secureRandom.uuid()}`;\n  }\n}\n";
    let output = ComponentScript::prepare(
        Path::new("/nonexistent"),
        source.as_bytes(),
        "client/services/StampService.js",
    )
    .expect("the page supplies the clock and secure random");
    let output = String::from_utf8(output).expect("utf8");
    assert!(
        output.contains(r#""field":"clock","name":"clock""#),
        "{output}"
    );
    assert!(
        output.contains(r#""field":"secureRandom","name":"secureRandom""#),
        "{output}"
    );
    assert!(!output.contains("@Needs"), "{output}");
}

#[test]
fn a_javascript_service_needs_every_curated_browser_api() {
    let names = [
        "connectivity",
        "pageVisibility",
        "preferences",
        "clipboard",
        "share",
        "notifications",
        "geolocation",
        "windowControls",
    ];
    let mut source = String::from("@Service\nexport default class DeviceService {\n");
    for name in names {
        let _ = write!(source, "  @Needs\n  {name};\n");
    }
    source.push_str("}\n");
    let output = ComponentScript::prepare(
        Path::new("/nonexistent"),
        source.as_bytes(),
        "client/services/DeviceService.js",
    )
    .expect("the page supplies every curated browser API");
    let output = String::from_utf8(output).expect("utf8");
    for name in names {
        let resource = format!(r#""field":"{name}","name":"{name}""#);
        assert!(output.contains(&resource), "{name}: {output}");
    }
}

#[test]
fn granted_browser_apis_reach_the_page_head_and_no_grant_adds_nothing() {
    let html = String::from("<html lang=\"en\"><head></head><body></body></html>");
    let mut site = crate::native::SiteSettings::default();
    let plain = super::page_head::PageHead::apply(&site, html.clone(), "/");
    assert!(!plain.contains("tachyon-browser-apis"), "{plain}");
    site.browser_apis = vec![String::from("clipboard"), String::from("share")];
    let granted = super::page_head::PageHead::apply(&site, html, "/");
    assert!(
        granted.contains(r#"<meta name="tachyon-browser-apis" content="clipboard share"></head>"#),
        "{granted}"
    );
}

#[test]
fn a_declared_yon_origin_reaches_the_page_head_and_none_adds_nothing() {
    let html = String::from("<html lang=\"en\"><head></head><body></body></html>");
    let mut site = crate::native::SiteSettings::default();
    let plain = super::page_head::PageHead::apply(&site, html.clone(), "/");
    assert!(!plain.contains("tachyon-yon-origin"), "{plain}");
    site.yon_origin = Some(String::from("https://api.example.com"));
    let declared = super::page_head::PageHead::apply(&site, html, "/");
    assert!(
        declared.contains(
            r#"<meta name="tachyon-yon-origin" content="https://api.example.com"></head>"#
        ),
        "{declared}"
    );
}
