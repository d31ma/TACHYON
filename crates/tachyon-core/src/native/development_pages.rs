use super::DevelopmentOrigin;

#[derive(Clone, Copy, Debug)]
pub(super) struct DevelopmentPages<'origin> {
    origin: Option<&'origin DevelopmentOrigin>,
}

impl<'origin> DevelopmentPages<'origin> {
    const PLACEHOLDER: &'static str = "__DEVELOPMENT_ORIGIN__";
    const LINUX: &'static str = include_str!("linux_development_pages.c");
    const LINUX_NONE: &'static str = "static gboolean tachyon_development_request(WebKitURISchemeRequest *request, const char *relative) { (void)request; (void)relative; return FALSE; }\n";
    const SWIFT: &'static str = include_str!("TacDevelopmentPages.swift");
    const SWIFT_NONE: &'static str = "private enum TacDevelopmentPages {\n    static func start(_ task: WKURLSchemeTask, _ path: String, _ policy: String) -> Bool { false }\n    static func stop(_ task: WKURLSchemeTask) {}\n}\n";

    const JAVA: &'static str = include_str!("android_development_pages.java");
    const JAVA_NONE: &'static str = "  private WebResourceResponse developmentResponse(String name, Uri target) { return null; }\n";

    const WINDOWS: &'static str = include_str!("windows_development_pages.c");
    const WINDOWS_NONE: &'static str = "static BOOL tachyon_development_response(ICoreWebView2WebResourceRequestedEventArgs *arguments, const char *uri, const char *relative) { (void)arguments; (void)uri; (void)relative; return FALSE; }\n";

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

    pub(super) fn swift(self) -> String {
        self.source(Self::SWIFT, Self::SWIFT_NONE)
    }

    pub(super) fn java(self) -> String {
        self.source(Self::JAVA, Self::JAVA_NONE)
    }

    pub(super) fn windows(self) -> String {
        self.source(Self::WINDOWS, Self::WINDOWS_NONE)
    }

    fn source(self, serving: &str, none: &str) -> String {
        self.origin.map_or_else(
            || String::from(none),
            |origin| serving.replace(Self::PLACEHOLDER, origin.origin()),
        )
    }
}
