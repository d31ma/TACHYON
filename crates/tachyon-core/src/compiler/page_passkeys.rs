use super::granted_script::GrantedScript;

pub(super) struct PagePasskeys;

impl PagePasskeys {
    pub(super) const PATH: &'static str = ".tachyon/tac-passkeys.js";
    pub(super) const SCRIPT: GrantedScript = GrantedScript {
        grant: "passkeys",
        script_tag: r#"<script type="module" src="/.tachyon/tac-passkeys.js" data-tachyon-runtime></script>"#,
        insecure_note: "Passkeys need a secure page: tac.origin is not https, so WebAuthn works there only on localhost during development (ADR 0126).",
        source: include_str!("../template/tac-passkeys.js"),
    };
}
