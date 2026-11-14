
#if os(macOS)
@MainActor
final class AppleCodeScannerView: NSObject {
    private static let size = NSRect(x: 0, y: 0, width: 560, height: 460)
    private static let escape = "\u{1b}"
    private let preview: AVCaptureVideoPreviewLayer
    private let reason: String
    private let cancelled: () -> Void
    private var sheet: NSWindow?

    init(_ preview: AVCaptureVideoPreviewLayer, _ reason: String, _ cancelled: @escaping () -> Void) {
        self.preview = preview
        self.reason = reason
        self.cancelled = cancelled
    }

    func show(over view: WKWebView) -> Bool {
        guard let parent = view.window else { return false }
        let sheet = NSWindow(contentRect: Self.size, styleMask: [.titled], backing: .buffered, defer: false)
        sheet.contentView = content()
        self.sheet = sheet
        parent.beginSheet(sheet)
        return true
    }

    func hide() {
        guard let sheet else { return }
        self.sheet = nil
        sheet.sheetParent?.endSheet(sheet)
    }

    private func content() -> NSView {
        let content = NSView(frame: Self.size)
        content.wantsLayer = true
        content.layer?.backgroundColor = NSColor.black.cgColor
        preview.frame = content.bounds
        preview.autoresizingMask = [.layerWidthSizable, .layerHeightSizable]
        content.layer?.addSublayer(preview)
        let label = NSTextField(wrappingLabelWithString: reason)
        label.textColor = .white
        label.alignment = .center
        let button = NSButton(title: "Cancel", target: self, action: #selector(cancel))
        button.keyEquivalent = Self.escape
        button.bezelStyle = .rounded
        for item in [label, button] as [NSView] {
            item.translatesAutoresizingMaskIntoConstraints = false
            content.addSubview(item)
        }
        NSLayoutConstraint.activate([
            label.topAnchor.constraint(equalTo: content.topAnchor, constant: 20),
            label.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 24),
            label.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -24),
            button.centerXAnchor.constraint(equalTo: content.centerXAnchor),
            button.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -20),
        ])
        return content
    }

    @objc private func cancel() {
        cancelled()
    }
}
#else
@MainActor
final class AppleCodeScannerView: UIViewController {
    private static let square: CGFloat = 240
    private static let margin: CGFloat = 24
    private let preview: AVCaptureVideoPreviewLayer
    private let reason: String
    private let cancelled: () -> Void

    init(_ preview: AVCaptureVideoPreviewLayer, _ reason: String, _ cancelled: @escaping () -> Void) {
        self.preview = preview
        self.reason = reason
        self.cancelled = cancelled
        super.init(nibName: nil, bundle: nil)
        modalPresentationStyle = .fullScreen
    }

    required init?(coder: NSCoder) {
        nil
    }

    override var keyCommands: [UIKeyCommand]? {
        [UIKeyCommand(input: UIKeyCommand.inputEscape, modifierFlags: [], action: #selector(cancel))]
    }

    func show(over view: WKWebView) -> Bool {
        guard var top = view.window?.rootViewController else { return false }
        while let next = top.presentedViewController { top = next }
        top.present(self, animated: true)
        return true
    }

    func hide() {
        dismiss(animated: true)
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        view.layer.addSublayer(preview)
        let frame = UIView()
        frame.layer.borderColor = UIColor.white.cgColor
        frame.layer.borderWidth = 3
        frame.layer.cornerRadius = 16
        let label = UILabel()
        label.text = reason
        label.textColor = .white
        label.numberOfLines = 0
        label.textAlignment = .center
        label.font = .preferredFont(forTextStyle: .headline)
        label.adjustsFontForContentSizeCategory = true
        var style = UIButton.Configuration.filled()
        style.title = "Cancel"
        style.baseBackgroundColor = .white
        style.baseForegroundColor = .black
        style.cornerStyle = .capsule
        let button = UIButton(configuration: style, primaryAction: UIAction { [weak self] _ in self?.cancel() })
        for item in [frame, label, button] {
            item.translatesAutoresizingMaskIntoConstraints = false
            view.addSubview(item)
        }
        let guide = view.safeAreaLayoutGuide
        NSLayoutConstraint.activate([
            frame.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            frame.centerYAnchor.constraint(equalTo: view.centerYAnchor),
            frame.widthAnchor.constraint(equalToConstant: Self.square),
            frame.heightAnchor.constraint(equalToConstant: Self.square),
            label.topAnchor.constraint(equalTo: guide.topAnchor, constant: Self.margin),
            label.leadingAnchor.constraint(equalTo: guide.leadingAnchor, constant: Self.margin),
            label.trailingAnchor.constraint(equalTo: guide.trailingAnchor, constant: -Self.margin),
            button.centerXAnchor.constraint(equalTo: view.centerXAnchor),
            button.bottomAnchor.constraint(equalTo: guide.bottomAnchor, constant: -Self.margin),
            button.heightAnchor.constraint(greaterThanOrEqualToConstant: 44),
            button.widthAnchor.constraint(greaterThanOrEqualToConstant: 120),
        ])
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        preview.frame = view.bounds
    }

    @objc private func cancel() {
        cancelled()
    }
}
#endif
