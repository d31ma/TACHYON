use super::DevelopmentOrigin;

#[derive(Clone, Copy, Debug)]
pub(super) struct DevelopmentPages<'origin> {
    origin: Option<&'origin DevelopmentOrigin>,
}

impl<'origin> DevelopmentPages<'origin> {
    const PLACEHOLDER: &'static str = "__DEVELOPMENT_ORIGIN__";
    const LINUX: &'static str = include_str!("linux_development_pages.c");
    const LINUX_NONE: &'static str = "static gboolean tachyon_development_request(WebKitURISchemeRequest *request, const char *relative) { (void)request; (void)relative; return FALSE; }\n";

    #[cfg(test)]
    pub(super) const fn none() -> Self {
        Self { origin: None }
    }

    pub(super) const fn serving(origin: Option<&'origin DevelopmentOrigin>) -> Self {
        Self { origin }
    }

    pub(super) fn linux(self) -> String {
        self.source(Self::LINUX, Self::LINUX_NONE)
    }

    fn source(self, serving: &str, none: &str) -> String {
        self.origin.map_or_else(
            || String::from(none),
            |origin| serving.replace(Self::PLACEHOLDER, origin.origin()),
        )
    }
}
