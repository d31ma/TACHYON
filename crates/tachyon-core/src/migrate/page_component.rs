#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PageComponent {
    pub(super) page: String,
    stem: String,
}

impl PageComponent {
    pub(super) const PAGES: &'static str = "client/pages";
    const ROOT_STEM: &'static str = "home";
    const TAG_BYTES: &'static [u8] = b"abcdefghijklmnopqrstuvwxyz0123456789-";

    pub(super) fn of(page: &str) -> Option<Self> {
        let rest = page.strip_prefix(Self::PAGES)?;
        (rest.is_empty() | rest.starts_with('/')).then_some(())?;
        let segments: Option<Vec<&str>> = rest
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(Self::segment)
            .collect();
        let stem = segments?.join("-");
        let named = [stem.as_str(), Self::ROOT_STEM][usize::from(stem.is_empty())];
        Some(Self {
            page: String::from(page),
            stem: String::from(named),
        })
    }

    pub(super) fn page_file(&self, name: &str) -> String {
        format!("{}/{name}", self.page)
    }

    pub(super) fn tag(&self) -> String {
        format!("page-{}", self.stem)
    }

    pub(super) fn view(&self) -> String {
        format!("client/components/page/{}/tac.html", self.stem)
    }

    pub(super) fn style(&self) -> String {
        format!("client/components/page/{}/tac.css", self.stem)
    }

    pub(super) fn document_style(&self) -> String {
        format!("client/shared/styles/{}-page.css", self.stem)
    }

    pub(super) fn script(&self) -> String {
        format!("client/shared/scripts/{}-page.js", self.stem)
    }

    pub(super) fn published(relative: &str) -> &str {
        relative.strip_prefix("client").unwrap_or(relative)
    }

    fn segment(raw: &str) -> Option<&str> {
        let segment = raw.strip_prefix('_').unwrap_or(raw);
        let starts = segment
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase());
        let tag_bytes = segment.bytes().all(|byte| Self::TAG_BYTES.contains(&byte));
        ([starts, tag_bytes] == [true, true]).then_some(segment)
    }
}
