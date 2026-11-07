struct FilePickerResult: Codable {
    let status: FilePickerStatus
    var handle: String? = nil
    var name: String? = nil
    var text: String? = nil
}
