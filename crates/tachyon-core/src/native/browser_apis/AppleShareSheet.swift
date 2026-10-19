@MainActor
final class AppleShareSheet: NSObject {
    private var reply: ((String) -> Void)?
    private var shown: NSObject?

    init(_ reply: @escaping (String) -> Void) {
        self.reply = reply
    }

    static func items(_ request: [String: Any]) -> [Any]? {
        var items: [Any] = []
        let words = [request["title"], request["text"]].compactMap { $0 as? String }.filter { !$0.isEmpty }
        if !words.isEmpty { items.append(words.joined(separator: "\n")) }
        if let link = request["url"] as? String, !link.isEmpty {
            guard let url = URL(string: link), url.scheme != nil else { return nil }
            items.append(url)
        }
        return items.isEmpty ? nil : items
    }

    func present(_ items: [Any], _ view: WKWebView) {
        let anchor = CGRect(x: view.bounds.midX, y: view.bounds.midY, width: 1, height: 1)
        #if os(macOS)
        let picker = NSSharingServicePicker(items: items)
        picker.delegate = self
        shown = picker
        picker.show(relativeTo: anchor, of: view, preferredEdge: .minY)
        #else
        guard var top = view.window?.rootViewController else {
            finish(AppleBrowserApis.failure("The share sheet has no window."))
            return
        }
        while let next = top.presentedViewController { top = next }
        let sheet = UIActivityViewController(activityItems: items, applicationActivities: nil)
        sheet.popoverPresentationController?.sourceView = view
        sheet.popoverPresentationController?.sourceRect = anchor
        sheet.completionWithItemsHandler = { [weak self] _, completed, _, error in
            MainActor.assumeIsolated { self?.finished(completed, error) }
        }
        shown = sheet
        top.present(sheet, animated: true)
        #endif
    }

    private func finished(_ completed: Bool, _ error: Error?) {
        if completed { finish(AppleBrowserApis.success(NSNull())); return }
        finish(AppleBrowserApis.failure(error?.localizedDescription ?? "Share was cancelled."))
    }

    private func finish(_ answer: String) {
        guard let reply else { return }
        self.reply = nil
        shown = nil
        reply(answer)
    }
}

#if os(macOS)
extension AppleShareSheet: NSSharingServicePickerDelegate {
    func sharingServicePicker(_ sharingServicePicker: NSSharingServicePicker, didChoose service: NSSharingService?) {
        finished(service != nil, nil)
    }
}
#endif
