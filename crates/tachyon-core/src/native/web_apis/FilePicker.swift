final class FilePicker {
    private let owner = TacPageCapabilities.constructing

    func available() -> Bool {
        answer("available", [:]).status == .available
    }

    func open(_ options: FilePickerOptions = FilePickerOptions()) -> FilePickerResult {
        guard !options.multiple, options.types.count <= 2, options.types.allSatisfy({ ["text/plain", "application/json"].contains($0) }) else {
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
        let text = TacPageCapabilities.answer(owner, "filePicker." + operation, fields)
        return (try? tacModel(FilePickerResult.self, tacParseJSON(text))) ?? FilePickerResult(status: .unavailable)
    }
}
