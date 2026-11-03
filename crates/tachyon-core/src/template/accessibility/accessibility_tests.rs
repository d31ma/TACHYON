use super::AccessibilityCheck;
use crate::template::TemplateFrontend;
use std::collections::BTreeSet;
use std::path::Path;

fn problems(view: &str) -> Vec<String> {
    let program = TemplateFrontend::compile(view, "client/pages/tac.html", &BTreeSet::new())
        .expect("the view compiles");
    AccessibilityCheck::program_diagnostics(&program)
        .iter()
        .map(|problem| format!("{} {}", problem.code, problem.message))
        .collect()
}

fn assert_clean(view: &str) {
    let found = problems(view);
    assert!(found.is_empty(), "{view}: {found:?}");
}

fn assert_refused(view: &str, part: &str) {
    let found = problems(view);
    assert!(
        found
            .iter()
            .any(|problem| problem.starts_with("TY1336") && problem.contains(part)),
        "{view}: {found:?}"
    );
}

#[test]
fn a_picture_needs_alt_text_unless_it_is_marked_as_decorative() {
    assert_refused(r#"<img src="/a.png">"#, "has no alt text");
    assert_refused(r#"<img src="/a.png" alt="">"#, "not marked as decorative");
    assert_clean(r#"<img src="/a.png" alt="A red bicycle">"#);
    assert_clean(r#"<img src="/a.png" :alt="product.name">"#);
    assert_clean(r#"<img src="/a.png" alt="" role="presentation">"#);
    assert_clean(r#"<img src="/a.png" alt="" aria-hidden="true">"#);
}

#[test]
fn buttons_and_links_need_an_accessible_name() {
    assert_refused("<button></button>", "<button> has no accessible name");
    assert_refused(
        r#"<a href="/cart"><img src="/cart.svg" alt="" role="none"></a>"#,
        "<a> has no accessible name",
    );
    assert_clean("<button>Save</button>");
    assert_clean("<button>{label}</button>");
    assert_clean(r#"<button aria-label="Close"></button>"#);
    assert_clean(r#"<a href="/" aria-labelledby="home-title"></a>"#);
    assert_clean(r#"<a href="/"><img src="/logo.svg" alt="Home"></a>"#);
    assert_clean(r#"<a href="/"><span><strong>Home</strong></span></a>"#);
    assert_clean(r#"<button><logic :if="open">Close</logic></button>"#);
    assert_clean("<a>Not a link without href</a>");
}

#[test]
fn a_form_field_needs_a_label() {
    assert_refused(
        r#"<input type="email" placeholder="Email">"#,
        "<input> has no label",
    );
    assert_refused(
        "<select><option>One</option></select>",
        "<select> has no label",
    );
    assert_refused("<textarea></textarea>", "<textarea> has no label");
    assert_clean(r#"<label>Email <input type="email"></label>"#);
    assert_clean(r#"<label for="email">Email</label><input id="email" type="email">"#);
    assert_clean(r#"<input id="email" type="email"><label for="email">Email</label>"#);
    assert_clean(r#"<label :for="field.id">Name</label><input :id="field.id">"#);
    assert_clean(r#"<input aria-label="Search">"#);
    assert_clean(r#"<input type="hidden" name="token"><input type="submit" value="Send">"#);
}

#[test]
fn headings_go_down_one_level_at_a_time() {
    assert_refused("<h1>Shop</h1><h3>Cart</h3>", "<h3> follows <h1>");
    assert_refused(
        "<section><h2>A</h2><div><h4>B</h4></div></section>",
        "<h4> follows <h2>",
    );
    assert_clean("<h2>Cart</h2><h3>Items</h3><h2>Total</h2><h3>Tax</h3>");
    assert_clean("<h3>A component may start at any level</h3>");
}

#[test]
fn positive_tabindex_is_refused() {
    assert_refused(r#"<div tabindex="2">Box</div>"#, "tabindex=\"2\"");
    assert_clean(r#"<div tabindex="0">Box</div><div tabindex="-1">Box</div>"#);
}

#[test]
fn aria_attributes_must_exist_have_valid_values_and_fit_the_element() {
    assert_refused(
        r#"<nav aria-lable="Main">Home</nav>"#,
        "aria-lable, which is not an ARIA attribute",
    );
    assert_refused(
        r#"<div aria-hidden="yes">x</div>"#,
        "aria-hidden takes true or false",
    );
    assert_refused(
        r#"<button aria-pressed="on">Bold</button>"#,
        "aria-pressed takes true or false or mixed",
    );
    assert_refused(
        r#"<button aria-hidden="true">Save</button>"#,
        "the keyboard can still reach it",
    );
    assert_refused(
        r#"<div aria-label="Card">x</div>"#,
        "without a role cannot have a name",
    );
    assert_clean(r#"<div role="region" aria-label="Card">x</div>"#);
    assert_clean(r#"<nav aria-label="Main"><a href="/" aria-current="page">Home</a></nav>"#);
    assert_clean(r#"<span aria-hidden="true">*</span><button :aria-expanded="open">Menu</button>"#);
    assert_clean(r#"<button aria-hidden="true" tabindex="-1">Save</button>"#);
    assert_clean(r#"<button aria-hidden="true" disabled>Save</button>"#);
}

#[test]
fn a_frame_needs_a_title() {
    assert_refused(r#"<iframe src="/map"></iframe>"#, "<iframe> has no title");
    assert_clean(r#"<iframe src="/map" title="Map of the store"></iframe>"#);
    assert_clean(r#"<iframe src="/report" aria-label="Report"></iframe>"#);
}

#[test]
fn the_check_reads_pages_and_components_only_when_the_client_switch_is_on() {
    let files = [
        (
            Path::new("client/pages/tac.html"),
            "<main><product-card></product-card><img src=\"/a.png\"></main>".as_bytes(),
        ),
        (
            Path::new("client/components/product/card/tac.html"),
            "<button></button>".as_bytes(),
        ),
    ];
    let found = AccessibilityCheck::diagnostics(
        files.into_iter(),
        crate::annotation_layout::RuleSwitch::every_realm(),
    );
    let paths: Vec<&str> = found
        .iter()
        .map(|problem| problem.spans[0].file.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "client/pages/tac.html",
            "client/components/product/card/tac.html"
        ]
    );
}

#[test]
fn the_build_refuses_an_inaccessible_view_when_the_switch_is_on() {
    let root = tempfile::tempdir().expect("temporary project");
    let pages = root.path().join("client/pages");
    std::fs::create_dir_all(&pages).expect("pages folder");
    std::fs::write(
        pages.join("tac.html"),
        "<main><img src=\"/a.png\"></main>\n",
    )
    .expect("page");
    let manifest = root.path().join("client/manifest.json");
    std::fs::write(&manifest, r#"{"tac":{"accessibility":{"check":false}}}"#).expect("manifest");
    crate::ProjectDiscovery::discover(root.path()).expect("the switch is off");
    std::fs::write(&manifest, r#"{"tac":{"accessibility":{"check":true}}}"#).expect("manifest");
    let failure = crate::ProjectDiscovery::discover(root.path())
        .expect_err("the picture without alt text stops the build");
    let codes: Vec<u16> = failure
        .diagnostics()
        .iter()
        .map(|problem| problem.code.number())
        .collect();
    assert_eq!(codes, [1336]);
}
