@MainActor
final class MacOsFilePicker {
    static let shared = MacOsFilePicker()
    static let byteLimit = 65536
    static let handleLimit = 128
    static let ownerLimit = 8
    static let lifetime: TimeInterval = 300
    private var files: [String: (owner: String, url: URL, expiry: Date, scoped: Bool)] = [:]
    private var panel: NSOpenPanel? = nil
    private var panelOwner: String? = nil
    var granted: Bool = false

    func call(_ operation: String, _ request: [String: Any]) -> String {
        guard let owner = request["owner"] as? String, UUID(uuidString: owner) != nil else { return answer(.invalid) }
        prune()
        if operation == "dispose" { dispose(owner); return answer(.disposed) }
        guard granted else { dispose(owner); return answer(.denied) }
        switch operation {
        case "available": return answer(.available)
        case "open": return open(owner, request)
        case "readText": return read(owner, request)
        case "release": return release(owner, request)
        default: return answer(.invalid)
        }
    }

    func disposeAll() {
        panel?.cancel(nil)
        for handle in Array(files.keys) { remove(handle) }
    }

    private func open(_ owner: String, _ request: [String: Any]) -> String {
        guard panel == nil, files.count < Self.handleLimit,
              files.values.filter({ $0.owner == owner }).count < Self.ownerLimit else { return answer(.unavailable) }
        guard let types = request["types"] as? [String], types.count <= 2,
              types.allSatisfy({ ["text/plain", "application/json"].contains($0) }) else { return answer(.invalid) }
        let picker = NSOpenPanel()
        picker.canChooseFiles = true
        picker.canChooseDirectories = false
        picker.allowsMultipleSelection = false
        if !types.isEmpty { picker.allowedContentTypes = types.compactMap { UTType(mimeType: $0) } }
        panel = picker
        panelOwner = owner
        let timeout = Timer.scheduledTimer(withTimeInterval: 30, repeats: false) { _ in
            MainActor.assumeIsolated { picker.cancel(nil) }
        }
        RunLoop.main.add(timeout, forMode: .modalPanel)
        defer { timeout.invalidate(); panel = nil; panelOwner = nil }
        guard picker.runModal() == .OK, let url = picker.url else { return answer(.cancelled) }
        guard granted else { return answer(.denied) }
        let scoped = url.startAccessingSecurityScopedResource()
        let handle = UUID().uuidString
        files[handle] = (owner, url, Date().addingTimeInterval(Self.lifetime), scoped)
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
        if panelOwner == owner { panel?.cancel(nil) }
        for (handle, file) in files where file.owner == owner { remove(handle) }
    }

    private func prune() {
        for (handle, file) in files where file.expiry <= Date() { remove(handle) }
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
