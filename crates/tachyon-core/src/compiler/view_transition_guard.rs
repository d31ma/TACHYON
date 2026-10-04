pub(crate) struct ViewTransitionGuard;

impl ViewTransitionGuard {
    const SCRIPT: &'static str = "\nfor (const type of ['pageswap', 'pagereveal']) addEventListener(type, (event) => { const transition = event.viewTransition; if (transition) for (const done of [transition.ready, transition.finished, transition.updateCallbackDone]) done.catch(() => {}) })\n";

    pub(crate) fn tag() -> String {
        format!("<script data-tachyon-runtime>{}</script>", Self::SCRIPT)
    }

    pub(crate) fn script_source() -> String {
        super::ContentSecurityPolicy::inline_script_hash(Self::SCRIPT)
    }
}

#[cfg(test)]
mod tests {
    use super::ViewTransitionGuard;

    #[test]
    fn a_skipped_view_transition_is_handled_on_both_pages() {
        let tag = ViewTransitionGuard::tag();
        assert!(tag.starts_with("<script data-tachyon-runtime>"), "{tag}");
        assert!(tag.contains("'pageswap', 'pagereveal'"), "{tag}");
        assert!(tag.contains("done.catch(() => {})"), "{tag}");
        assert!(ViewTransitionGuard::script_source().starts_with("'sha256-"));
    }
}
