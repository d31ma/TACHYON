@MainActor
final class AppleMediaCapture {
    static let scheme = "tachyon-app"
    static let host = "bundle"
    static let mediaGrants = ["camera", "microphone", "screen"]

    static func prepare(_ configuration: WKWebViewConfiguration, _ granted: [String]) {
        guard Self.mediaGrants.contains(where: granted.contains) else { return }
        configuration.mediaTypesRequiringUserActionForPlayback = []
        Self.playInline(configuration)
    }

    static func decision(_ origin: WKSecurityOrigin, _ type: WKMediaCaptureType, _ granted: [String]) -> WKPermissionDecision {
        guard Self.isOwnPage(origin) else { return .deny }
        return Self.kinds(type).allSatisfy(granted.contains) ? .grant : .deny
    }

    private static func isOwnPage(_ origin: WKSecurityOrigin) -> Bool {
        origin.protocol == Self.scheme && origin.host == Self.host && origin.port == 0
    }

    private static func kinds(_ type: WKMediaCaptureType) -> [String] {
        switch type {
        case .camera: return ["camera"]
        case .microphone: return ["microphone"]
        case .cameraAndMicrophone: return ["camera", "microphone"]
        @unknown default: return ["unknown"]
        }
    }

    private static func playInline(_ configuration: WKWebViewConfiguration) {
        #if os(iOS)
        configuration.allowsInlineMediaPlayback = true
        #endif
    }
}
