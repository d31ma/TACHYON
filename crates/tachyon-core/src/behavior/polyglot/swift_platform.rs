pub(crate) struct SwiftPlatform;

impl SwiftPlatform {
    pub(super) const FOUNDATION: &'static str = concat!(
        include_str!("swift_foundation/BehaviorPlatform.swift"),
        include_str!("swift_foundation/BehaviorLock.swift")
    );

    pub(crate) fn in_browser(program: &str, browser: &str) -> String {
        program.replace(Self::FOUNDATION, browser)
    }
}
