use super::RoutePath;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct MountPath {
    prefix: String,
}

impl MountPath {
    const MAX_SEGMENTS: usize = 4;
    const RESERVED_SEGMENT: &'static str = "shared";
    const ORIGIN_FILES: [&'static str; 5] = [
        "tachyon-sw.js",
        ".tachyon/register-sw.js",
        "manifest.webmanifest",
        "robots.txt",
        "sitemap.xml",
    ];
    const ORIGIN_FOLDER: &'static str = ".well-known";
    const DEPLOY_FILES: [&'static str; 3] = [
        "route-manifest.json",
        "api.json",
        ".tachyon/build-state.json",
    ];
    const DEPLOY_FOLDER: &'static str = "api-docs";
    const RUNTIME_ATTRIBUTES: [&'static str; 2] = ["src=\"/.tachyon/", "href=\"/.tachyon/"];
    const ATTRIBUTE_START: &'static str = "=\"";
    const SERVICES_FOLDER: &'static str = "/.tachyon/services/";
    const INVALID: &'static str = "Manifest tac.mount must be a path prefix such as /chat: one to four lowercase segments of letters, digits and single hyphens, with no trailing slash and no _parameter segment.";
    const RESERVED: &'static str =
        "Manifest tac.mount cannot start with /shared: that folder holds the site's shared files.";

    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let segments = Self::segments(value)?;
        Self::check_count(&segments)?;
        Self::check_static(&segments)?;
        Self::check_reserved(&segments)?;
        Ok(Self {
            prefix: String::from(value),
        })
    }

    pub(crate) fn prefix(&self) -> &str {
        &self.prefix
    }

    pub(crate) fn is_root(&self) -> bool {
        self.prefix.is_empty()
    }

    pub(crate) fn route(&self, route: &str) -> String {
        Self::without_trailing_slash(&format!("{}{route}", self.prefix))
    }

    pub(crate) fn services_folder(&self) -> String {
        format!("{}{}", self.prefix, Self::SERVICES_FOLDER)
    }

    pub(crate) fn document(&self, html: &str) -> String {
        Self::RUNTIME_ATTRIBUTES
            .iter()
            .fold(String::from(html), |text, attribute| {
                self.mounted_attribute(&text, attribute)
            })
    }

    pub(crate) fn files(&self, files: Vec<(PathBuf, Vec<u8>)>) -> Vec<(PathBuf, Vec<u8>)> {
        files
            .into_iter()
            .filter(|(path, _)| self.kept(path))
            .map(|(path, bytes)| (self.placed(path), bytes))
            .collect()
    }

    fn segments(value: &str) -> Result<Vec<&str>, String> {
        value
            .strip_prefix('/')
            .map(|rest| rest.split('/').collect())
            .ok_or_else(|| String::from(Self::INVALID))
    }

    fn check_count(segments: &[&str]) -> Result<(), String> {
        (segments.len() <= Self::MAX_SEGMENTS)
            .then_some(())
            .ok_or_else(|| String::from(Self::INVALID))
    }

    fn check_static(segments: &[&str]) -> Result<(), String> {
        Self::all_static(segments)
            .then_some(())
            .ok_or_else(|| String::from(Self::INVALID))
    }

    fn all_static(segments: &[&str]) -> bool {
        segments
            .iter()
            .all(|segment| RoutePath::valid_static_segment(segment))
    }

    fn check_reserved(segments: &[&str]) -> Result<(), String> {
        (segments.first() != Some(&Self::RESERVED_SEGMENT))
            .then_some(())
            .ok_or_else(|| String::from(Self::RESERVED))
    }

    fn without_trailing_slash(joined: &str) -> String {
        let trimmed = joined.trim_end_matches('/');
        if trimmed.is_empty() {
            return String::from("/");
        }
        String::from(trimmed)
    }

    fn mounted_attribute(&self, text: &str, attribute: &str) -> String {
        let mounted_start = format!("{}{}", Self::ATTRIBUTE_START, self.prefix);
        let mounted = attribute.replacen(Self::ATTRIBUTE_START, &mounted_start, 1);
        text.replace(attribute, &mounted)
    }

    fn kept(&self, path: &Path) -> bool {
        let owned_by_origin = [
            Self::origin_file(path),
            path.starts_with(Self::ORIGIN_FOLDER),
        ];
        [self.is_root(), !owned_by_origin.contains(&true)].contains(&true)
    }

    fn origin_file(path: &Path) -> bool {
        Self::ORIGIN_FILES
            .iter()
            .any(|file| path == Path::new(file))
    }

    fn placed(&self, path: PathBuf) -> PathBuf {
        if self.stays(&path) {
            return path;
        }
        self.folder().join(path)
    }

    fn stays(&self, path: &Path) -> bool {
        [
            Self::deploy_file(path),
            path.starts_with(Self::DEPLOY_FOLDER),
            path.starts_with(self.folder()),
        ]
        .contains(&true)
    }

    fn deploy_file(path: &Path) -> bool {
        Self::DEPLOY_FILES
            .iter()
            .any(|file| path == Path::new(file))
    }

    fn folder(&self) -> PathBuf {
        PathBuf::from(self.prefix.trim_start_matches('/'))
    }
}
