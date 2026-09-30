final class FilePicker {
    private let owner: String
    private let call: (String, String) -> String

    init(owner: String, call: @escaping (String, String) -> String) {
        self.owner = owner
        self.call = call
    }

    func available() -> Bool {
        answer("available", [:]).status == .available
    }

    func open(_ options: FilePickerOptions = FilePickerOptions()) -> FilePickerResult {
        guard !options.multiple, options.types.allSatisfy({ ["text/plain", "application/json"].contains($0) }) else {
            return FilePickerResult(status: .invalid)
        }
        return answer("open", ["types": options.types])
    }

    func readText(_ handle: String) -> FilePickerResult {
        answer("readText", ["handle": handle])
    }

    func release(_ handle: String) -> FilePickerResult {
        answer("release", ["handle": handle])
    }

    private func answer(_ operation: String, _ fields: [String: Any]) -> FilePickerResult {
        guard NativeCapabilities.active(owner) else { return FilePickerResult(status: .disposed) }
        var request = fields
        request["owner"] = owner
        guard let data = try? JSONSerialization.data(withJSONObject: request),
              let text = String(data: data, encoding: .utf8),
              let result = try? JSONDecoder().decode(FilePickerResult.self, from: Data(call("filePicker." + operation, text).utf8)) else {
            return FilePickerResult(status: .unavailable)
        }
        return result
    }
}
