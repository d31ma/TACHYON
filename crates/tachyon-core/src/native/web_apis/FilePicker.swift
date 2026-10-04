final class FilePicker {
    func available() -> Bool {
        false
    }

    func open(_ options: FilePickerOptions = FilePickerOptions()) -> FilePickerResult {
        guard !options.multiple, options.types.allSatisfy({ ["text/plain", "application/json"].contains($0) }) else {
            return FilePickerResult(status: .invalid)
        }
        return FilePickerResult(status: .unsupported)
    }

    func readText(_ handle: String) -> FilePickerResult {
        FilePickerResult(status: .unsupported)
    }

    func release(_ handle: String) -> FilePickerResult {
        FilePickerResult(status: .unsupported)
    }
}
