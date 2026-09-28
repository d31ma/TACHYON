#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DesignRule {
    Dependencies,
    Names,
    Signatures,
    Constants,
    Visibility,
    Types,
    Tests,
}

impl DesignRule {
    pub(super) const ALL: [Self; 7] = [
        Self::Dependencies,
        Self::Names,
        Self::Signatures,
        Self::Constants,
        Self::Visibility,
        Self::Types,
        Self::Tests,
    ];
    const CODES: [u16; 7] = [1327, 1328, 1329, 1330, 1333, 1356, 1339];
    const SWITCH_NAMES: [&'static str; 7] = [
        "dependencies",
        "names",
        "signatures",
        "constants",
        "visibility",
        "types",
        "tests",
    ];
    const FEATURES: [&'static str; 7] = [
        "dependencies.direction",
        "names.plain_english",
        "signatures.size",
        "constants.named",
        "visibility.private_when_unused",
        "types.every_member",
        "tests.assert_without_logic",
    ];
    const ON_BY_DEFAULT: [bool; 7] = [true, true, true, true, true, false, false];

    pub(super) const fn code(self) -> u16 {
        Self::CODES[self as usize]
    }

    pub(super) const fn switch_name(self) -> &'static str {
        Self::SWITCH_NAMES[self as usize]
    }

    pub(super) const fn feature(self) -> &'static str {
        Self::FEATURES[self as usize]
    }

    pub(super) const fn on_by_default(self) -> bool {
        Self::ON_BY_DEFAULT[self as usize]
    }
}
