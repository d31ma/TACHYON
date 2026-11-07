pub(crate) struct PageFile;

impl PageFile {
    pub(crate) const PATH: &'static str = "client/pages/tac.html";
    const OPENING: &'static str = "<app-shell>\n  <app-hero />\n";
    const CLOSING: &'static str = "</app-shell>\n";

    pub(crate) fn text(components: &[String]) -> String {
        let lines: String = components.iter().map(|tag| Self::line(tag)).collect();
        format!("{}{lines}{}", Self::OPENING, Self::CLOSING)
    }

    pub(crate) fn with_component(page: &str, tag: &str) -> Option<String> {
        let place = Self::free_place(page, tag)?;
        Some(format!(
            "{}{}{}",
            &page[..place],
            Self::line(tag),
            &page[place..]
        ))
    }

    pub(crate) fn line(tag: &str) -> String {
        format!("  <{tag} hydrate=\"load\" />\n")
    }

    fn free_place(page: &str, tag: &str) -> Option<usize> {
        if page.contains(&format!("<{tag} ")) {
            return None;
        }
        page.rfind(Self::CLOSING.trim_end())
    }
}
