import AppKit
import WebKit

fileprivate extension AppDelegate {
    func fixtureView() -> WKWebView { webView! }
    func fixtureWindow() -> NSWindow { window! }
    func fixtureAssets() -> NativeUiStyleAssets { liveStyleHost.assets }
    func fixtureSession() -> NativeUiStyleSession { liveStyleHost.session! }
}
