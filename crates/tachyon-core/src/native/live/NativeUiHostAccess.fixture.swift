import AppKit
import WebKit

extension AppDelegate {
    func fixtureFrameControl() -> NativeUiHostFrameFixture { fixtureFrame }
    func fixtureView() -> WKWebView { webView! }
    func fixtureWindow() -> NSWindow { window! }
    func fixtureAssets() -> NativeUiStyleAssets { liveStyleHost.assets }
    func fixtureSession() -> NativeUiStyleSession { liveStyleHost.session! }
}
