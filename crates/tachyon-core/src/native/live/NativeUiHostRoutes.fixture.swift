import Foundation

@MainActor struct NativeUiHostRoutesFixture {
    static func check(_ assets: NativeUiStyleAssets) throws {
        let scheme = NativeUiStyleHostScheme(assets, tachyonDocument)
        try routes(scheme, assets)
        try denied(scheme, assets)
        try documents(assets)
        print("native-ui-host-route-controls-passed")
    }

    static func selectedDocument(_ path: String) -> String? {
        tachyonRoute(path).flatMap { tachyonRouteDocuments[$0] }
    }

    private static func documents(_ assets: NativeUiStyleAssets) throws {
        let selection = NativeUiStyleDocumentSelection(assets, selectedDocument)
        let allowed = [("/?draft=1", "index.html"), ("/status/?draft=1", "status/index.html"), ("/topic/telemetry/?draft=1", "topic/_slug/index.html"), ("/topic/telemetry?draft=1", "topic/_slug/index.html"), ("/index.html?draft=1&label=%CE%BB", "index.html")]
        try allowed.forEach { try document(selection, $0) }
        let denied = ["/unknown?draft=1", "/styles/theme.css?draft=1", "/status?draft=1#section", "/status#", "/status%2findex.html?draft=1", "//status?draft=1", "/status//?draft=1", "/status/../status?draft=1", "/status/./?draft=1", "/%2e/status?draft=1", "/status/%2e%2e/status?draft=1", "/topic/telemetry/extra.html?draft=1", "/.tachyon-native-ui/" + assets.current.descriptor.generation + "/index.html?draft=1", "/index.html?" + String(repeating: "x", count: 2049)]
        try denied.forEach { try documentRefused(selection, "tachyon-app://bundle" + $0) }
        try ["https://bundle/status?draft=1", "tachyon-app://other/status?draft=1", "tachyon-app://user@bundle/status?draft=1", "tachyon-app://bundle:80/status?draft=1"].forEach { try documentRefused(selection, $0) }
    }

    private static func document(_ selection: NativeUiStyleDocumentSelection, _ expected: (String, String)) throws {
        let observed = try selection.document(URL(string: "tachyon-app://bundle" + expected.0)!)
        try NativeUiRuntimeFailure.require(observed == expected.1, "Queried document selected a different captured route")
    }

    private static func documentRefused(_ selection: NativeUiStyleDocumentSelection, _ source: String) throws {
        try NativeUiRuntimeFailure.require(documentFailure(selection, source) != nil, "Untrusted queried document selected styles: " + source)
    }

    private static func documentFailure(_ selection: NativeUiStyleDocumentSelection, _ source: String) -> Error? {
        do { _ = try selection.document(URL(string: source)!); return nil } catch { return error }
    }

    private static func routes(_ scheme: NativeUiStyleHostScheme, _ assets: NativeUiStyleAssets) throws {
        let routes = [
            ("/?draft=1", "index.html"), ("/status/?draft=1#section", "status/index.html"),
            ("/topic/telemetry?draft=1", "topic/_slug/index.html"), ("/topic/telemetry/detail.txt", "topic/_slug/detail.txt"), ("/topic/telemetry/extra.html", "topic/_slug/extra.html"),
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
