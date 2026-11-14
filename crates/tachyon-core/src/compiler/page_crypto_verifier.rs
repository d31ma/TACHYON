pub(super) struct PageCryptoVerifier;

impl PageCryptoVerifier {
    pub(super) const PATH: &'static str = ".tachyon/tac-ed25519.js";
    const SOURCE: &'static str = include_str!("../template/vendor/noble-ed25519.js");
    const NOTICE: &'static str = "// noble-ed25519 3.2.0, Copyright (c) 2019 Paul Miller, https://github.com/paulmillr/noble-ed25519\n// Licensed under the MIT License. Tachyon loads it only to verify an Ed25519 signature over an empty message (ADR 0133).\n";

    pub(super) fn script() -> Vec<u8> {
        [Self::NOTICE, Self::SOURCE].concat().into_bytes()
    }
}
