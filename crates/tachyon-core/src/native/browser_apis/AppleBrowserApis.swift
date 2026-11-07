import CoreLocation
import UserNotifications
import WebKit

@MainActor
final class AppleBrowserApis: NSObject, UNUserNotificationCenterDelegate {
    static let shared = AppleBrowserApis()
    static let bridged = ["clipboard", "share", "notifications", "geolocation"]
    static let notAnswered = "The native app could not run that browser call."
    static let notificationsRefused = "Notifications are not allowed. Call requestPermission first."
    var granted: [String] = []
    private var permission = "default"
    private var share: AppleShareSheet?
    private lazy var position = AppleCurrentPosition()

    func prepare(_ view: WKWebView, _ load: @escaping () -> Void) {
        guard granted.contains("notifications") else { load(); return }
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        Task { @MainActor in
            let word = Self.word(await center.notificationSettings().authorizationStatus)
            self.permission = word
            if word != "default" { self.announce(word, view) }
            load()
        }
    }

    func call(_ capability: String, _ request: [String: Any], _ view: WKWebView?, _ reply: @escaping (String) -> Void) {
        let parts = capability.split(separator: ".").map(String.init)
        guard parts.count == 3, Self.bridged.contains(parts[1]) else { reply(Self.failure(Self.notAnswered)); return }
        guard granted.contains(parts[1]) else {
            reply(Self.failure("The \(parts[1]) browser API is not granted. Add \"\(parts[1])\" to tac.browserApis in client/manifest.json."))
            return
        }
        switch capability {
        case "browserApi.clipboard.writeText": reply(writeText(request))
        case "browserApi.clipboard.readText": reply(Self.success(readText()))
        case "browserApi.share.open": open(request, view, reply)
        case "browserApi.notifications.requestPermission": requestPermission(view, reply)
        case "browserApi.notifications.show": show(request, reply)
        case "browserApi.geolocation.currentPosition": position.find(Self.timeout(request), reply)
        default: reply(Self.failure(Self.notAnswered))
        }
    }

    nonisolated func userNotificationCenter(
        _ center: UNUserNotificationCenter, willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        [.banner, .list, .sound]
    }

    nonisolated static func success(_ value: Any) -> String {
        encode(["value": value])
    }

    nonisolated static func failure(_ message: String) -> String {
        encode(["error": message])
    }

    private func writeText(_ request: [String: Any]) -> String {
        guard let text = request["text"] as? String else { return Self.failure("Clipboard text must be a string.") }
        #if os(macOS)
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
        #else
        UIPasteboard.general.string = text
        #endif
        return Self.success(NSNull())
    }

    private func readText() -> String {
        #if os(macOS)
        return NSPasteboard.general.string(forType: .string) ?? ""
        #else
        return UIPasteboard.general.string ?? ""
        #endif
    }

    private func open(_ request: [String: Any], _ view: WKWebView?, _ reply: @escaping (String) -> Void) {
        guard share == nil else { reply(Self.failure("A share sheet is already open.")); return }
        guard let view else { reply(Self.failure("The share sheet has no window.")); return }
        guard let items = AppleShareSheet.items(request) else {
            reply(Self.failure("Share needs a title, text or an absolute URL."))
            return
        }
        let sheet = AppleShareSheet { [weak self] answer in
            self?.share = nil
            reply(answer)
        }
        share = sheet
        sheet.present(items, view)
    }

    private func requestPermission(_ view: WKWebView?, _ reply: @escaping (String) -> Void) {
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        Task { @MainActor in
            _ = try? await center.requestAuthorization(options: [.alert, .sound])
            let word = Self.word(await center.notificationSettings().authorizationStatus)
            if word != self.permission { self.permission = word; self.announce(word, view) }
            reply(Self.success(word))
        }
    }

    private func show(_ request: [String: Any], _ reply: @escaping (String) -> Void) {
        guard permission == "granted" else { reply(Self.failure(Self.notificationsRefused)); return }
        guard let title = request["title"] as? String else { reply(Self.failure("A notification needs a title.")); return }
        let body = request["body"] as? String ?? ""
        Task { @MainActor in
            let content = UNMutableNotificationContent()
            content.title = String(title.prefix(256))
            content.body = String(body.prefix(1024))
            let notification = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
            do {
                try await UNUserNotificationCenter.current().add(notification)
                reply(Self.success(NSNull()))
            } catch {
                reply(Self.failure(error.localizedDescription))
            }
        }
    }

    private func announce(_ word: String, _ view: WKWebView?) {
        view?.configuration.userContentController.addUserScript(WKUserScript(
            source: "globalThis.__tachyonNotificationPermission = '" + word + "'",
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true
        ))
    }

    private static func word(_ status: UNAuthorizationStatus) -> String {
        switch status {
        case .notDetermined: return "default"
        case .denied: return "denied"
        default: return "granted"
        }
    }

    private static func timeout(_ request: [String: Any]) -> TimeInterval {
        let milliseconds = (request["timeout"] as? NSNumber)?.doubleValue ?? 10000
        guard milliseconds.isFinite else { return 10 }
        return min(max(milliseconds, 1), 120000) / 1000
    }

    nonisolated private static func encode(_ object: [String: Any]) -> String {
        guard let data = try? JSONSerialization.data(withJSONObject: object) else {
            return "{\"error\":\"The native app could not answer.\"}"
        }
        return String(decoding: data, as: UTF8.self)
    }
}
