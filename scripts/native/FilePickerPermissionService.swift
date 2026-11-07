@Service
@NeedsAll
final class ImportService {
    private let filePicker: FilePicker
    private var handle: String = ""

    func choose() -> String {
        let answer = filePicker.open(FilePickerOptions(types: ["text/plain"]))
        handle = answer.handle ?? ""
        return answer.status.rawValue
    }

    func read() -> String { filePicker.readText(handle).status.rawValue }
    func text() -> String { filePicker.readText(handle).text ?? "" }
}
