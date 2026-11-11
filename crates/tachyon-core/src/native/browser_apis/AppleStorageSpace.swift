@MainActor
enum AppleStorageSpace {
    static let notGranted = "The persistentStorage browser API is not granted. Add \"persistentStorage\" to tac.browserApis in client/manifest.json."

    static func answer(_ call: String, _ granted: [String]) -> String {
        switch call {
        case "estimate": return estimate()
        case "persist": return persist(granted)
        default: return AppleBrowserApis.failure(AppleBrowserApis.notAnswered)
        }
    }

    private static func estimate() -> String {
        let space: [String: Any] = ["quota": quota(), "persisted": true]
        return AppleBrowserApis.success(space)
    }

    private static func persist(_ granted: [String]) -> String {
        guard granted.contains("persistentStorage") else { return AppleBrowserApis.failure(notGranted) }
        return AppleBrowserApis.success(true)
    }

    private static func quota() -> Int64 {
        let home = URL(fileURLWithPath: NSHomeDirectory())
        let values = try? home.resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey])
        return values?.volumeAvailableCapacityForImportantUsage ?? 0
    }
}
