#if os(macOS)
import UniformTypeIdentifiers

@MainActor
final class AppleRecorderFiles {
    static let shared = AppleRecorderFiles()
    static let mostChunkBytes = 49152
    static let mostFileName = 128
    static let mostRecordingName = 64
    static let formats = ["mp4", "webm"]
    static let nameLetters = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.-")
    static let unsafeFileLetters = CharacterSet(charactersIn: "/\\:*?\"<>|").union(.controlCharacters)
    static let outOfSpace = [Int(ENOSPC), NSFileWriteOutOfSpaceError]
    private var files: [String: FileHandle] = [:]
    private var choosing = false

    func answer(_ capability: String, _ request: [String: Any], _ view: WKWebView?, _ reply: @escaping (String) -> Void) {
        switch capability {
        case "browserApi.recorder.open": open(request, view, reply)
        case "browserApi.recorder.write": reply(write(request))
        case "browserApi.recorder.close": reply(close(request))
        default: reply(AppleBrowserApis.failure(AppleBrowserApis.notAnswered))
        }
    }

    private func open(_ request: [String: Any], _ view: WKWebView?, _ reply: @escaping (String) -> Void) {
        guard let recording = Self.recording(request), let name = Self.fileName(request), let format = Self.format(request) else {
            reply(AppleBrowserApis.failure("A recording call has a bad argument (invalid).")); return
        }
        guard !choosing else { reply(AppleBrowserApis.failure("A save dialog is already open (invalid).")); return }
        guard let window = view?.window else { reply(AppleBrowserApis.failure("The save dialog has no window (failed).")); return }
        let panel = NSSavePanel()
        panel.nameFieldStringValue = name
        panel.allowedContentTypes = [UTType(filenameExtension: format)].compactMap { $0 }
        panel.canCreateDirectories = true
        panel.directoryURL = FileManager.default.urls(for: .moviesDirectory, in: .userDomainMask).first
        choosing = true
        panel.beginSheetModal(for: window) { [weak self] response in
            self?.choosing = false
            let url = response == .OK ? panel.url : nil
            reply(self?.create(recording, url) ?? AppleBrowserApis.failure(AppleBrowserApis.notAnswered))
        }
    }

    private func create(_ recording: String, _ url: URL?) -> String {
        guard let url else { return AppleBrowserApis.failure("The save dialog was closed (cancelled).") }
        try? files.removeValue(forKey: recording)?.close()
        guard FileManager.default.createFile(atPath: url.path, contents: nil),
              let handle = try? FileHandle(forWritingTo: url) else {
            return AppleBrowserApis.failure("The recording file could not be made there (denied).")
        }
        files[recording] = handle
        return AppleBrowserApis.success(["fileName": url.lastPathComponent])
    }

    private func write(_ request: [String: Any]) -> String {
        guard let handle = Self.recording(request).flatMap({ files[$0] }) else {
            return AppleBrowserApis.failure("There is no such recording (unknown).")
        }
        guard let position = Self.position(request), let data = Self.chunk(request) else {
            return AppleBrowserApis.failure("A recording call has a bad argument (invalid).")
        }
        do {
            try handle.seek(toOffset: position)
            try handle.write(contentsOf: data)
            return AppleBrowserApis.success(NSNull())
        } catch {
            return AppleBrowserApis.failure(Self.writeFailure(error))
        }
    }

    private func close(_ request: [String: Any]) -> String {
        guard let handle = Self.recording(request).flatMap({ files.removeValue(forKey: $0) }) else {
            return AppleBrowserApis.failure("There is no such recording (unknown).")
        }
        do {
            try handle.synchronize()
            try handle.close()
            return AppleBrowserApis.success(NSNull())
        } catch {
            return AppleBrowserApis.failure(Self.writeFailure(error))
        }
    }

    private static func recording(_ request: [String: Any]) -> String? {
        guard let name = request["recording"] as? String, (1...mostRecordingName).contains(name.count) else { return nil }
        return name.unicodeScalars.allSatisfy(nameLetters.contains) ? name : nil
    }

    private static func fileName(_ request: [String: Any]) -> String? {
        guard let name = request["fileName"] as? String, (1...mostFileName).contains(name.count) else { return nil }
        return name.rangeOfCharacter(from: unsafeFileLetters) == nil ? name : nil
    }

    private static func format(_ request: [String: Any]) -> String? {
        guard let format = request["format"] as? String else { return nil }
        return formats.contains(format) ? format : nil
    }

    private static func position(_ request: [String: Any]) -> UInt64? {
        guard let number = request["position"] as? NSNumber else { return nil }
        return UInt64(exactly: number.doubleValue)
    }

    private static func chunk(_ request: [String: Any]) -> Data? {
        guard let text = request["data"] as? String, let data = Data(base64Encoded: text) else { return nil }
        return (1...mostChunkBytes).contains(data.count) ? data : nil
    }

    private static func writeFailure(_ error: Error) -> String {
        guard outOfSpace.contains((error as NSError).code) else { return "The recording could not be written (failed)." }
        return "There is no disk space left for the recording (full)."
    }
}
#endif
