use sha2::{Digest, Sha256};

pub(crate) struct ViewTransitionGuard;

impl ViewTransitionGuard {
    const OPENING: &'static str = "<script data-tachyon-runtime>";
    const SCRIPT: &'static str = "\nfor (const type of ['pageswap', 'pagereveal']) addEventListener(type, (event) => { const transition = event.viewTransition; if (transition) for (const done of [transition.ready, transition.finished, transition.updateCallbackDone]) done.catch(() => {}) })\n";
    const CLOSING: &'static str = "</script>";

    pub(crate) fn tag() -> String {
        format!("{}{}{}", Self::OPENING, Self::SCRIPT, Self::CLOSING)
    }

    pub(crate) fn script_source() -> String {
        let digest = Sha256::digest(Self::SCRIPT.as_bytes());
        format!("'sha256-{}'", crate::tac_web::base64(&digest))
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
