import Foundation

@MainActor struct NativeUiHostRoutesFixture {
    static func check(_ assets: NativeUiStyleAssets) throws {
        let scheme = NativeUiStyleHostScheme(assets, tachyonDocument)
        try routes(scheme, assets)
        try denied(scheme, assets)
        print("native-ui-host-route-controls-passed")
    }

    private static func routes(_ scheme: NativeUiStyleHostScheme, _ assets: NativeUiStyleAssets) throws {
        let routes = [
            ("/?draft=1", "index.html"), ("/status/?draft=1#section", "status/index.html"),
            ("/topic/telemetry?draft=1", "topic/_slug/index.html"), ("/topic/telemetry/detail.txt", "topic/_slug/detail.txt"),
            ("/status", "status/index.html"), ("/styles/theme.css?version=1", "styles/theme.css"),
        ]
        try routes.forEach { try route(scheme, assets, $0) }
    }

    private static func route(_ scheme: NativeUiStyleHostScheme, _ assets: NativeUiStyleAssets, _ route: (String, String)) throws {
        let url = URL(string: "tachyon-app://bundle" + route.0)!
        let resolved = try scheme.resolve(url)
        try NativeUiRuntimeFailure.require(try assets.asset(resolved) == assets.current.asset(route.1), "Generated captured route changed")
    }

    private static func denied(_ scheme: NativeUiStyleHostScheme, _ assets: NativeUiStyleAssets) throws {
        let urls = [
            "https://bundle/status", "tachyon-app://other/status", "tachyon-app://user@bundle/status", "tachyon-app://bundle:80/status",
            "tachyon-app://bundle/styles%2ftheme.css", "tachyon-app://bundle/styles%5ctheme.css", "tachyon-app://bundle/unknown",
            "tachyon-app://bundle/.tachyon-native-ui/" + String(repeating: "b", count: 64) + "/styles/theme.css?version=1",
        ]
        try urls.forEach { try refused(scheme, assets, $0) }
    }

    private static func refused(_ scheme: NativeUiStyleHostScheme, _ assets: NativeUiStyleAssets, _ source: String) throws {
        try NativeUiRuntimeFailure.require(failure(scheme, assets, source) != nil, "Untrusted host asset request was accepted")
    }

    private static func failure(_ scheme: NativeUiStyleHostScheme, _ assets: NativeUiStyleAssets, _ source: String) -> Error? {
        do { _ = try assets.asset(scheme.resolve(URL(string: source)!)); return nil } catch { return error }
    }
}
