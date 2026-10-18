use crate::native::{BrowserApiGrants, PageMetadata, SiteSettings, YonMember};

pub(crate) struct PageHead;

impl PageHead {
    const HEAD_END: &'static str = "</head>";
    const RUNTIME_SCRIPT: &'static str = r#"<script type="module" src="/.tachyon/tac-client.js""#;

    pub(super) fn apply(site: &SiteSettings, html: String, route: &str) -> String {
        let html = Self::with_route_metadata(site, html, route);
        let html = Self::with_manifest_head(site, html);
        let html = Self::with_browser_api_grants(site, html);
        let html = Self::with_yon_origin(site, html);
        let html = Self::with_styles(site, &html);
        let html = Self::with_blocking_scripts(site, &html);
        Self::with_scripts(site, &html)
    }

    pub(super) fn inject_before(html: &str, close: &str, value: &str) -> String {
        let position = html.rfind(close).unwrap_or(html.len());
        Self::insert_at(html, position, value)
    }

    fn inject_before_runtime(html: &str, value: &str) -> String {
        let runtime = html.find(Self::RUNTIME_SCRIPT);
        let position = runtime.or_else(|| html.rfind(Self::HEAD_END));
        Self::insert_at(html, position.unwrap_or(html.len()), value)
    }

    fn insert_at(html: &str, position: usize, value: &str) -> String {
        format!("{}{value}{}", &html[..position], &html[position..])
    }

    pub(crate) fn escape(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    fn with_route_metadata(site: &SiteSettings, html: String, route: &str) -> String {
        match site.metadata.for_route(route) {
            Some(declared) => Self::with_declared(&declared, &html),
            None => html,
        }
    }

    fn with_declared(declared: &PageMetadata, html: &str) -> String {
        let html = Self::inject_before(html, Self::HEAD_END, &Self::render_metadata(declared));
        Self::with_language(declared.lang.as_deref(), html)
    }

    fn with_language(lang: Option<&str>, html: String) -> String {
        match lang {
            Some(lang) => {
                let tag = format!("<html lang=\"{}\">", Self::escape(lang));
                html.replacen("<html lang=\"en\">", &tag, 1)
            }
            None => html,
        }
    }

    fn with_manifest_head(site: &SiteSettings, html: String) -> String {
        if site.manifest_head.is_empty() {
            return html;
        }
        Self::inject_before(&html, Self::HEAD_END, &site.manifest_head)
    }

    fn with_browser_api_grants(site: &SiteSettings, html: String) -> String {
        if site.browser_apis.is_empty() {
            return html;
        }
        let tag = BrowserApiGrants::meta_tag(&site.browser_apis);
        Self::inject_before(&html, Self::HEAD_END, &tag)
    }

    fn with_yon_origin(site: &SiteSettings, html: String) -> String {
        match site.yon_origin.as_deref() {
            Some(origin) => {
                Self::inject_before(&html, Self::HEAD_END, &YonMember::meta_tag(origin))
            }
            None => html,
        }
    }

    fn with_styles(site: &SiteSettings, html: &str) -> String {
        let links = Self::joined_tags(&site.styles, Self::style_link);
        Self::inject_before(html, Self::HEAD_END, &links)
    }

    fn with_blocking_scripts(site: &SiteSettings, html: &str) -> String {
        let scripts = Self::joined_tags(&site.blocking_scripts, Self::blocking_script);
        Self::inject_before_runtime(html, &scripts)
    }

    fn with_scripts(site: &SiteSettings, html: &str) -> String {
        let scripts = Self::joined_tags(&site.scripts, Self::module_script);
        Self::inject_before(html, Self::HEAD_END, &scripts)
    }

    fn joined_tags(sources: &[String], tag: fn(&str) -> String) -> String {
        sources.iter().map(|source| tag(source)).collect()
    }

    fn style_link(source: &str) -> String {
        format!(
            r#"<link rel="stylesheet" href="{}" data-tachyon-runtime>"#,
            Self::escape(source)
        )
    }

    fn blocking_script(source: &str) -> String {
        format!(
            r#"<script src="{}" data-tachyon-runtime></script>"#,
            Self::escape(source)
        )
    }

    fn module_script(source: &str) -> String {
        format!(
            r#"<script type="module" src="{}" data-tachyon-runtime></script>"#,
            Self::escape(source)
        )
    }

    fn render_metadata(declared: &PageMetadata) -> String {
        let parts = [
            declared.title.as_deref().map(Self::title_tags),
            declared.description.as_deref().map(Self::description_tags),
            declared.canonical.as_deref().map(Self::canonical_tags),
            declared.image.as_deref().map(Self::image_tags),
            declared.site_name.as_deref().map(Self::site_name_tags),
            Self::robots_tag(declared),
        ];
        parts.into_iter().flatten().collect()
    }

    fn title_tags(title: &str) -> String {
        let title = Self::escape(title);
        format!(
            "<title>{title}</title>\
             <meta property=\"og:title\" content=\"{title}\">\
             <meta name=\"twitter:title\" content=\"{title}\">"
        )
    }

    fn description_tags(description: &str) -> String {
        let description = Self::escape(description);
        format!(
            "<meta name=\"description\" content=\"{description}\">\
             <meta property=\"og:description\" content=\"{description}\">\
             <meta name=\"twitter:description\" content=\"{description}\">"
        )
    }

    fn canonical_tags(canonical: &str) -> String {
        let canonical = Self::escape(canonical);
        format!(
            "<link rel=\"canonical\" href=\"{canonical}\">\
             <meta property=\"og:url\" content=\"{canonical}\">"
        )
    }

    fn image_tags(image: &str) -> String {
        let image = Self::escape(image);
        format!(
            "<meta property=\"og:image\" content=\"{image}\">\
             <meta name=\"twitter:card\" content=\"summary_large_image\">"
        )
    }

    fn site_name_tags(site: &str) -> String {
        let site = Self::escape(site);
        format!(
            "<meta property=\"og:site_name\" content=\"{site}\">\
             <meta property=\"og:type\" content=\"website\">"
        )
    }

    fn robots_tag(declared: &PageMetadata) -> Option<String> {
        declared
            .is_hidden_from_search()
            .then(|| String::from("<meta name=\"robots\" content=\"noindex, follow\">"))
    }
}
