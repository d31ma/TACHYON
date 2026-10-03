use super::AuthoredDomGuard;
use std::path::Path;

struct DomGuardFixture;

impl DomGuardFixture {
    fn accepted(path: &str, source: &str) {
        AuthoredDomGuard::check(Path::new(path), source)
            .unwrap_or_else(|failure| panic!("{source}: {failure}"));
    }

    fn refused(path: &str, source: &str, needle: &str) {
        let failure = AuthoredDomGuard::check(Path::new(path), source)
            .err()
            .unwrap_or_else(|| panic!("accepted {source}"));
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.to_string(), "TY1364");
        let span = &diagnostic.spans[0];
        assert_eq!(span.file, path);
        let found = &source[span.start as usize..span.end as usize];
        assert!(found.contains(needle), "{span:?}: {found}");
        assert!(
            diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("tac.html") && help.contains("tac.css"))
        );
    }
}

#[test]
fn authored_dom_guard_refuses_markup_writes_and_literal_bracket_members() {
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "element.innerHTML = '<p>wrong</p>';",
        "innerHTML",
    );
    DomGuardFixture::refused(
        "client/helpers/ViewHelper.ts",
        "element['outerHTML'] += html;",
        "outerHTML",
    );
    DomGuardFixture::refused(
        "client/shared/assets/renamed.mjs",
        "element [ 'innerHTML' ] = source;",
        "innerHTML",
    );
    DomGuardFixture::refused(
        "client/remotes/ViewRemote.js",
        "element.insertAdjacentHTML('beforeend', source);",
        "insertAdjacentHTML",
    );
}

#[test]
fn authored_dom_guard_follows_document_aliases_and_split_assignments() {
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const root = window; const doc = root.document; const alias = doc; alias.write(markup);",
        "write",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.ts",
        "let doc: Document; doc = document; const alias: Document = doc; alias.writeln(markup);",
        "writeln",
    );
    DomGuardFixture::refused(
        "client/shared/utils/MarkupUtils.js",
        "let node; node = document.createElement('article'); const alias = node; document.body.append(alias);",
        "append",
    );
    DomGuardFixture::refused(
        "client/components/notice/tac.js",
        "document.body.append(document.createElement('article'));",
        "append",
    );
}

#[test]
fn authored_dom_guard_follows_created_instance_fields_and_namespaced_nodes() {
    DomGuardFixture::refused(
        "client/services/IconService.js",
        "this.icon = document.createElementNS(namespace, 'svg'); document.body.appendChild(this.icon);",
        "appendChild",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const doc = document; const node = doc.createElement('style'); document.head.insertBefore(node, anchor);",
        "insertBefore",
    );
}

#[test]
fn authored_dom_guard_refuses_style_writes_and_stylesheet_aliases() {
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const style = element.style; const alias = style; alias.cssText = css;",
        "cssText",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const style = element.style; style.display = 'grid';",
        "display",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "element.style.setProperty('width', width);",
        "setProperty",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "element.setAttribute('style', css);",
        "setAttribute",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "let sheet; sheet = new CSSStyleSheet(); const rules = sheet; rules.replace(css);",
        "replace",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const sheet = document.styleSheets[0]; sheet.insertRule(css);",
        "insertRule",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "document.adoptedStyleSheets = [sheet];",
        "adoptedStyleSheets",
    );
}

#[test]
fn authored_dom_guard_allows_text_state_class_theme_and_inert_source() {
    DomGuardFixture::accepted(
        "client/services/ViewService.js",
        "element.textContent = '<p>data</p>'; element.classList.add('ready'); state.markup = '<p>data</p>'; document.documentElement.setAttribute('data-theme', 'dark'); localStorage.setItem('theme', 'dark');",
    );
    DomGuardFixture::accepted(
        "client/shared/utils/TextUtils.js",
        "// element.innerHTML = html;\nconst sample = \"element.outerHTML = html\"; const expression = /document.write(html)/; const docs = `element.insertAdjacentHTML('beforeend', html)`;",
    );
    DomGuardFixture::accepted(
        "client/shared/utils/TextUtils.ts",
        "const text = 'plain'; const alias = text; alias.replace('a', 'b'); object.write(value); if (element.innerHTML === cached) return;",
    );
    DomGuardFixture::accepted(
        "client/services/DataService.js",
        "const model = { innerHTML: 'inert data' }; model.innerHTML = '<p>data</p>'; const document = { write: () => 1 }; document.write('data');",
    );
}

#[test]
fn authored_dom_guard_tracks_alias_reassignment_and_block_shadowing() {
    DomGuardFixture::accepted(
        "client/services/ViewService.js",
        "let doc = document; doc = model; doc.write(value); const sheet = 'data'; sheet.replace('a', 'b');",
    );
    DomGuardFixture::accepted(
        "client/services/ViewService.js",
        "const sheet = new CSSStyleSheet(); { const sheet = 'data'; sheet.replace('a', 'b'); }",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const sheet = new CSSStyleSheet(); { const sheet = 'data'; sheet.replace('a', 'b'); } sheet.replace(css);",
        "replace",
    );
}

#[test]
fn authored_dom_guard_leaves_other_realms_languages_and_generated_intakes_unchanged() {
    DomGuardFixture::accepted(
        "server/controllers/ViewController.js",
        "element.innerHTML = markup;",
    );
    DomGuardFixture::accepted(
        "client/services/ViewService.rs",
        "element.innerHTML = markup;",
    );
    DomGuardFixture::accepted("tools/build.js", "document.write(markup);");
    DomGuardFixture::accepted("dist/.tachyon/tac-client.js", "element.innerHTML = markup;");
}

#[test]
fn authored_dom_guard_tracks_callable_sink_aliases_and_node_factories() {
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const output = document.write.bind(document); output(markup);",
        "output",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.ts",
        "let replace; const sheet = new CSSStyleSheet(); replace = sheet.replace.bind(sheet); replace(css);",
        "replace",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const create = document.createElement.bind(document); const node = create('article'); const insert = document.body.append.bind(document.body); insert(node);",
        "insert",
    );
    DomGuardFixture::accepted(
        "client/services/TextService.js",
        "const text = 'safe'; const replace = text.replace.bind(text); replace('a', 'b');",
    );
}

#[test]
fn authored_dom_guard_follows_semicolon_free_document_and_created_node_aliases() {
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const doc = document\ndoc.write(markup)",
        "write",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const node = document.createElement('article')\ndocument.body.append(node)",
        "append",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.ts",
        "let doc: Document\ndoc = document\nconst alias: Document = doc\nalias.writeln(markup)",
        "writeln",
    );
}

#[test]
fn authored_dom_guard_keeps_balanced_multiline_alias_rhs_and_continuations() {
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const node = document\n .createElement(\n 'article'\n )\ndocument.body.append(node)",
        "append",
    );
    DomGuardFixture::refused(
        "client/services/ViewService.js",
        "const doc = (\n document\n)\ndoc.write(markup)",
        "write",
    );
    DomGuardFixture::accepted(
        "client/services/DataService.js",
        "const data = {\n innerHTML: 'safe data',\n nested: { markup: '<p>data</p>' }\n}\ndata.innerHTML = '<p>data</p>'\nconst text = [\n 'a',\n 'b'\n].join('')\ntext.replace('a', 'b')",
    );
    DomGuardFixture::accepted(
        "client/services/ThemeService.js",
        "const root = document\n .documentElement\nroot.setAttribute(\n 'data-theme',\n 'dark'\n)",
    );
}

#[test]
fn authored_dom_guard_lexical_refusals_retain_actual_typescript_and_module_paths() {
    let typed = AuthoredDomGuard::check(
        Path::new("client/services/ViewService.ts"),
        "const text = 'unterminated",
    )
    .err()
    .unwrap_or_else(|| panic!("accepted incomplete typed source"));
    assert!(typed.to_string().contains("cannot be checked"), "{typed}");
    assert_eq!(
        typed.diagnostics()[0].spans[0].file,
        "client/services/ViewService.ts"
    );
    let module = AuthoredDomGuard::check(
        Path::new("client/shared/ordinary.mjs"),
        "/* unclosed source",
    )
    .err()
    .unwrap_or_else(|| panic!("accepted incomplete module source"));
    assert!(module.to_string().contains("cannot be checked"), "{module}");
    assert_eq!(
        module.diagnostics()[0].spans[0].file,
        "client/shared/ordinary.mjs"
    );
}
