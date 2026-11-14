
@MainActor
final class AppleCodeScanner: NSObject {
    static let busy = "A code scan is already open."
    static let reasonKey = "NSCameraUsageDescription"
    private var reply: ((String) -> Void)?
    private var capture: AppleCodeCapture?
    private var screen: AppleCodeScannerView?

    static var present: Bool {
        AVCaptureDevice.default(for: .video) != nil
    }

    func scan(_ view: WKWebView?, _ reply: @escaping (String) -> Void) {
        guard self.reply == nil else { reply(AppleBrowserApis.failure(Self.busy)); return }
        guard let view, Self.present else { reply(Self.failed("unsupported")); return }
        self.reply = reply
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized: open(view)
        case .notDetermined: ask(view)
        default: finish(Self.failed("denied"))
        }
    }

    func cancel(_ reply: @escaping (String) -> Void) {
        finish(Self.failed("cancelled"))
        reply(AppleBrowserApis.success(NSNull()))
    }

    private func ask(_ view: WKWebView) {
        AVCaptureDevice.requestAccess(for: .video) { allowed in
            Task { @MainActor in
                guard allowed else { self.finish(Self.failed("denied")); return }
                self.open(view)
            }
        }
    }

    private func open(_ view: WKWebView) {
        guard reply != nil else { return }
        guard let device = AVCaptureDevice.default(for: .video),
              let capture = AppleCodeCapture(device, found: { [weak self] text in
                  Task { @MainActor in self?.finish(Self.scanned(text)) }
              }) else {
            finish(Self.failed("failed"))
            return
        }
        let screen = AppleCodeScannerView(capture.preview(), Self.reason()) { [weak self] in
            self?.finish(Self.failed("cancelled"))
        }
        guard screen.show(over: view) else { finish(Self.failed("failed")); return }
        self.capture = capture
        self.screen = screen
        capture.start()
    }

    private func finish(_ answer: String) {
        guard let reply else { return }
        self.reply = nil
        capture?.stop()
        capture = nil
        screen?.hide()
        screen = nil
        reply(answer)
    }

    private static func reason() -> String {
        Bundle.main.object(forInfoDictionaryKey: reasonKey) as? String ?? ""
    }

    static func scanned(_ text: String) -> String {
        AppleBrowserApis.success(["CodeScanned", ["text": text]] as [Any])
    }

    static func failed(_ reason: String) -> String {
        AppleBrowserApis.success(["ScanFailed", ["reason": reason]] as [Any])
    }
}
