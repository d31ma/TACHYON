enum FilePickerStatus: String, Codable {
    case available, selected, cancelled, read, released, denied, unsupported, unavailable, invalid, disposed, tooLarge
}
