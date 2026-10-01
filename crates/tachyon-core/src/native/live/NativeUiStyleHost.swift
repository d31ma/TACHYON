import Foundation
import WebKit

@MainActor final class NativeUiStyleHost {
    let assets: NativeUiStyleAssets
    private var selection: NativeUiStyleDocumentSelection?
    private(set) var session: NativeUiStyleSession?

    init(_ installed: NativeUiRuntimeGeneration) { assets = NativeUiStyleAssets(installed) }

    func configure(_ configuration: WKWebViewConfiguration, _ document: @escaping (String) -> String?) {
        selection = NativeUiStyleDocumentSelection(assets, document)
        configuration.setURLSchemeHandler(NativeUiStyleHostScheme(assets, document), forURLScheme: "tachyon-app")
    }

    func bind(_ view: WKWebView) throws { try bound(view) }

    func invalidate() { invalidating() }

    private func bound(_ view: WKWebView) throws {
        try NativeUiRuntimeFailure.require(session == nil, "Native style host already owns a view")
        session = NativeUiStyleSession(view, assets, selection)
    }

    private func invalidating() {
        guard let session else { return }
        let request = session.beginInvalidation()
        Task { await session.finishInvalidation(request) }
    }
}
