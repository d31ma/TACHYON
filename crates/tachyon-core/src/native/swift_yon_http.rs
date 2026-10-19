use super::host::quoted_string_escape;

pub(super) struct SwiftYonHttp;

impl SwiftYonHttp {
    const TRANSPORT: &'static str = include_str!("swift_yon_http/BehaviorYonHttp.swift");
    const ORIGIN_PLACEHOLDER: &'static str = "__TAC_YON_ORIGIN__";
    const NETWORK_IMPORT: &'static str =
        "#if canImport(FoundationNetworking)\nimport FoundationNetworking\n#endif\n";

    pub(super) fn linked(program: &str, yon_origin: Option<&str>) -> String {
        yon_origin.map_or_else(
            || program.to_owned(),
            |origin| Self::reaching(program, origin),
        )
    }

    fn reaching(program: &str, origin: &str) -> String {
        let transport =
            Self::TRANSPORT.replace(Self::ORIGIN_PLACEHOLDER, &quoted_string_escape(origin));
        let reaching = crate::behavior::SwiftPlatform::reaching_yon(program, &transport);
        format!("{}{reaching}", Self::NETWORK_IMPORT)
    }
}

#[cfg(test)]
mod tests {
    use super::SwiftYonHttp;

    #[test]
    fn a_native_swift_program_without_a_yon_origin_is_unchanged() {
        let program = "throw BehaviorError(\"HTTP_MISSING\")";
        assert_eq!(SwiftYonHttp::linked(program, None), program);
    }

    #[test]
    fn a_native_swift_program_with_a_yon_origin_sends_its_exchanges_there() {
        let program = concat!(
            include_str!("../behavior/polyglot/swift_foundation/BehaviorPlatform.swift"),
            include_str!("../behavior/polyglot/swift_foundation/BehaviorLock.swift")
        );
        let linked = SwiftYonHttp::linked(program, Some("https://api.example.com"));
        assert!(
            linked.contains("try await BehaviorYonHttp.send(request)"),
            "{linked}"
        );
        assert!(
            linked.contains("static let origin = \"https://api.example.com\""),
            "{linked}"
        );
        assert!(!linked.contains("HTTP_MISSING\")\n    }"), "{linked}");
    }
}
