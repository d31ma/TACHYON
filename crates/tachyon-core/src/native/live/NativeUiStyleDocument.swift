import Foundation
import WebKit

@MainActor struct NativeUiStyleDocument {
    let view: WKWebView
    private let selection: NativeUiStyleDocumentSelection?

    init(_ view: WKWebView, _ selection: NativeUiStyleDocumentSelection?) {
        self.view = view
        self.selection = selection
    }

    func url() throws -> URL {
        let url = try ownedURL()
        try selected(url)
        return url
    }

    private func ownedURL() throws -> URL {
        guard let url = view.url else { throw NativeUiRuntimeFailure(description: "No owned main-frame document") }
        return url
    }

    private func selected(_ url: URL) throws {
        switch selection {
        case .some(let selection): _ = try selection.document(url)
        case .none: _ = try NativeUiStyleAddress.path(url)
        }
    }
}
