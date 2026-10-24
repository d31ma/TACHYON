@MainActor
final class IosFilePicker: NSObject, UIDocumentPickerDelegate {
    static let shared = IosFilePicker()
    static let byteLimit = 65536
    static let handleLimit = 128
    static let ownerLimit = 8
    static let lifetime: TimeInterval = 300
    static let dialogSeconds: TimeInterval = 30
    private(set) var files: [String: (owner: String, url: URL, expiry: Date, scoped: Bool)] = [:]
    private var picker: UIDocumentPickerViewController? = nil
    private var pickerOwner: String? = nil
    private var chosen: ((URL?) -> Void)? = nil
    weak var presenter: UIViewController?
    var granted: Bool = false
    var clock: () -> Date = Date.init

    func call(_ operation: String, _ request: [String: Any], _ reply: @escaping (String) -> Void) {
        guard operation == "open", let owner = request["owner"] as? String, UUID(uuidString: owner) != nil, granted else {
            reply(callNow(operation, request))
            return
        }
        prune()
        open(owner, request, reply)
    }

    func callNow(_ operation: String, _ request: [String: Any]) -> String {
        guard let owner = request["owner"] as? String, UUID(uuidString: owner) != nil else { return answer(.invalid) }
        prune()
        if operation == "dispose" { dispose(owner); return answer(.disposed) }
        guard granted else { dispose(owner); return answer(.denied) }
        switch operation {
        case "available": return answer(.available)
        case "open": return openNow(owner, request)
        case "readText": return read(owner, request)
        case "release": return release(owner, request)
        default: return answer(.invalid)
        }
    }

    func disposeAll() {
        finish(nil)
        for handle in Array(files.keys) { remove(handle) }
    }

    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        guard controller === picker else { return }
        finish(urls.first)
    }

    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        guard controller === picker else { return }
        finish(nil)
    }

    private func openNow(_ owner: String, _ request: [String: Any]) -> String {
        var result: String? = nil
        open(owner, request) { result = $0 }
        while result == nil {
            RunLoop.main.run(mode: .default, before: Date(timeIntervalSinceNow: 0.05))
        }
        return result ?? answer(.unavailable)
    }

    private func open(_ owner: String, _ request: [String: Any], _ reply: @escaping (String) -> Void) {
        guard picker == nil, files.count < Self.handleLimit,
              files.values.filter({ $0.owner == owner }).count < Self.ownerLimit,
              let presenter = topmost() else { reply(answer(.unavailable)); return }
        guard let types = request["types"] as? [String], types.count <= 2,
              types.allSatisfy({ ["text/plain", "application/json"].contains($0) }) else { reply(answer(.invalid)); return }
        let contentTypes = types.isEmpty ? [UTType.item] : types.compactMap { UTType(mimeType: $0) }
        let controller = UIDocumentPickerViewController(forOpeningContentTypes: contentTypes, asCopy: false)
        controller.allowsMultipleSelection = false
        controller.delegate = self
        picker = controller
        pickerOwner = owner
        let timeout = Timer.scheduledTimer(withTimeInterval: Self.dialogSeconds, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated { self?.finish(nil) }
        }
        chosen = { [weak self] url in
            timeout.invalidate()
            guard let self else { reply("{\"status\":\"unavailable\"}"); return }
            guard let url else { reply(self.answer(.cancelled)); return }
            guard self.granted else { reply(self.answer(.denied)); return }
            reply(self.keep(owner, url, scoped: url.startAccessingSecurityScopedResource()))
        }
        presenter.present(controller, animated: true)
    }

    private func topmost() -> UIViewController? {
        var top = presenter
        while let shown = top?.presentedViewController, !shown.isBeingDismissed { top = shown }
        return top?.viewIfLoaded?.window == nil ? nil : top
    }

    private func finish(_ url: URL?) {
        guard let done = chosen else { return }
        let shown = picker
        chosen = nil
        picker = nil
        pickerOwner = nil
        if url == nil, let shown, shown.presentingViewController != nil, !shown.isBeingDismissed {
            shown.dismiss(animated: true)
        }
        done(url)
    }

    func keep(_ owner: String, _ url: URL, scoped: Bool) -> String {
        let handle = UUID().uuidString
        files[handle] = (owner, url, clock().addingTimeInterval(Self.lifetime), scoped)
        return encode(FilePickerResult(status: .selected, handle: handle, name: url.lastPathComponent))
    }

    private func read(_ owner: String, _ request: [String: Any]) -> String {
        guard let handle = request["handle"] as? String, let file = files[handle], file.owner == owner else { return answer(.invalid) }
        do {
            let stream = try FileHandle(forReadingFrom: file.url)
            defer { try? stream.close() }
            let data = try stream.read(upToCount: Self.byteLimit + 1) ?? Data()
            guard data.count <= Self.byteLimit else { return answer(.tooLarge) }
            guard let text = String(data: data, encoding: .utf8) else { return answer(.invalid) }
            return encode(FilePickerResult(status: .read, text: text))
        } catch { return answer(.unavailable) }
    }

    private func release(_ owner: String, _ request: [String: Any]) -> String {
        guard let handle = request["handle"] as? String, let file = files[handle], file.owner == owner else { return answer(.invalid) }
        remove(handle)
        return answer(.released)
    }

    private func dispose(_ owner: String) {
        if pickerOwner == owner { finish(nil) }
        for (handle, file) in files where file.owner == owner { remove(handle) }
    }

    private func prune() {
        for (handle, file) in files where file.expiry <= clock() { remove(handle) }
    }

    private func remove(_ handle: String) {
        guard let file = files.removeValue(forKey: handle) else { return }
        if file.scoped { file.url.stopAccessingSecurityScopedResource() }
    }

    private func answer(_ status: FilePickerStatus) -> String {
        encode(FilePickerResult(status: status))
    }

    private func encode(_ value: FilePickerResult) -> String {
        guard let data = try? JSONEncoder().encode(value) else { return "{\"status\":\"unavailable\"}" }
        return String(decoding: data, as: UTF8.self)
    }
}
