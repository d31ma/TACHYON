@Service
@NeedsAll
final class PickerService {
    private let filePicker: FilePicker
    var status: String = "Ready"

    func choose() {
        let chosen = filePicker.open(FilePickerOptions(types: ["text/plain"]))
        status = readSelected(chosen)
    }

    private func readSelected(_ chosen: FilePickerResult) -> String {
        guard let handle = chosen.handle else { return chosen.status.rawValue }
        let content = filePicker.readText(handle)
        _ = filePicker.release(handle)
        return content.status.rawValue
    }
}
