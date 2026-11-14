use super::granted_script::GrantedScript;

pub(super) struct PageLocks;

impl PageLocks {
    pub(super) const PATH: &'static str = ".tachyon/tac-locks.js";
    pub(super) const SCRIPT: GrantedScript = GrantedScript {
        grant: "locks",
        script_tag: r#"<script type="module" src="/.tachyon/tac-locks.js" data-tachyon-runtime></script>"#,
        insecure_note: "Locks need a secure page: tac.origin is not https, so the browser has no Web Locks there and every tab gets each lock at once (ADR NNNN).",
        source: include_str!("../template/tac-locks.js"),
    };
}
