pub(super) struct NativeUiStylesSources;

impl NativeUiStylesSources {
    const STYLES: [&str; 11] = [
        include_str!("live/NativeUiCssMode.swift"),
        include_str!("live/NativeUiCssMask.swift"),
        include_str!("live/NativeUiCssProfile.swift"),
        include_str!("live/NativeUiCssGrammar.swift"),
        include_str!("live/NativeUiStyleAddress.swift"),
        include_str!("live/NativeUiStyleAssets.swift"),
        include_str!("live/NativeUiStyleScheme.swift"),
        include_str!("live/NativeUiStyleTicket.swift"),
        include_str!("live/NativeUiStylePhase.swift"),
        include_str!("live/NativeUiStyleReply.swift"),
        include_str!("live/NativeUiStyleCall.swift"),
    ];
    const SCRIPT: [&'static str; 5] = [
        include_str!("live/NativeUiStyleAttributes.js"),
        include_str!("live/NativeUiStyleOriginal.js"),
        include_str!("live/NativeUiStyleLink.js"),
        include_str!("live/NativeUiStyleController.js"),
        include_str!("live/NativeUiStyleInvocation.js"),
    ];

    pub(super) fn sources() -> impl Iterator<Item = (String, &'static str)> {
        Self::captured().chain(Self::styles())
    }

    fn captured() -> impl Iterator<Item = (String, &'static str)> {
        super::native_ui_runtime_tests::NativeUiRuntimeFixture::SOURCES
            .into_iter()
            .map(|(name, source)| (name.to_owned(), source))
    }

    fn styles() -> impl Iterator<Item = (String, &'static str)> {
        Self::STYLES
            .into_iter()
            .enumerate()
            .map(|(position, source)| (format!("StylePart{position}.swift"), source))
    }

    pub(super) fn script() -> String {
        Self::SCRIPT.join("\n")
    }

    pub(super) fn swift_script() -> String {
        include_str!("live/NativeUiStyleScript.swift")
            .replace("__STYLE_TRANSACTION_JS__", &Self::script())
    }
}
