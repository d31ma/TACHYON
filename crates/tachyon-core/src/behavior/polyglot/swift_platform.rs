pub(crate) struct SwiftPlatform;

impl SwiftPlatform {
    pub(super) const FOUNDATION: &'static str = concat!(
        include_str!("swift_foundation/BehaviorPlatform.swift"),
        include_str!("swift_foundation/BehaviorLock.swift")
    );

    const MISSING_EXCHANGE: &'static str = "    static func exchange(_ request: BehaviorJSON) async throws -> BehaviorJSON {\n        throw BehaviorError(\"HTTP_MISSING\")\n    }\n";
    const YON_EXCHANGE: &'static str = "    static func exchange(_ request: BehaviorJSON) async throws -> BehaviorJSON {\n        try await BehaviorYonHttp.send(request)\n    }\n";

    pub(crate) fn in_browser(program: &str, browser: &str) -> String {
        program.replace(Self::FOUNDATION, browser)
    }

    pub(crate) fn reaching_yon(program: &str, transport: &str) -> String {
        let platform = Self::FOUNDATION.replace(Self::MISSING_EXCHANGE, Self::YON_EXCHANGE);
        program.replace(Self::FOUNDATION, &format!("{platform}{transport}"))
    }
}

#[cfg(test)]
mod tests {
    use super::SwiftPlatform;

    #[test]
    fn the_foundation_platform_holds_the_exchange_a_native_app_replaces() {
        assert!(SwiftPlatform::FOUNDATION.contains(SwiftPlatform::MISSING_EXCHANGE));
        let linked =
            SwiftPlatform::reaching_yon(SwiftPlatform::FOUNDATION, "final class Transport {}\n");
        assert!(!linked.contains("HTTP_MISSING"), "{linked}");
        assert!(linked.contains("BehaviorYonHttp.send(request)"), "{linked}");
        assert!(linked.ends_with("final class Transport {}\n"), "{linked}");
    }
}
