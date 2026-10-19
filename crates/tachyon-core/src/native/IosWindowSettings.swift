@MainActor
private enum IosWindowSettings {
    static let integrated = __WINDOW_INTEGRATED__
    static let fullscreen = __WINDOW_FULLSCREEN__
    static let themeColor: UIColor? = __WINDOW_THEME_COLOR__
    static let backgroundColor: UIColor? = __WINDOW_BACKGROUND_COLOR__
    static let statusBarStyle: UIStatusBarStyle = __WINDOW_STATUS_BAR_STYLE__

    static func prepare(_ root: UIView, _ webView: WKWebView) {
        root.backgroundColor = themeColor ?? backgroundColor ?? .systemBackground
        if let backgroundColor {
            webView.isOpaque = false
            webView.backgroundColor = backgroundColor
            webView.scrollView.backgroundColor = backgroundColor
        }
        if integrated {
            webView.scrollView.contentInsetAdjustmentBehavior = .never
        }
    }

    static func topAnchor(_ root: UIView) -> NSLayoutYAxisAnchor {
        integrated ? root.topAnchor : root.safeAreaLayoutGuide.topAnchor
    }

    static func insets(_ root: UIView?) -> String {
        guard let root else { return "{\"height\":0,\"leading\":0,\"trailing\":0}" }
        let edges = root.safeAreaInsets
        let rightToLeft = root.effectiveUserInterfaceLayoutDirection == .rightToLeft
        let leading = rightToLeft ? edges.right : edges.left
        let trailing = rightToLeft ? edges.left : edges.right
        return "{\"height\":\(Int(edges.top.rounded())),\"leading\":\(Int(leading.rounded())),\"trailing\":\(Int(trailing.rounded()))}"
    }

    static func publish(_ root: UIView, _ webView: WKWebView?) {
        guard integrated, let webView else { return }
        webView.evaluateJavaScript("globalThis.__tachyonTitleBar?.apply(" + insets(root) + ")")
    }
}
