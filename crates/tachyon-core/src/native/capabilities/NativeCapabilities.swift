enum NativeCapabilities {
    nonisolated(unsafe) static var call: (String, String) -> String = { _, _ in "{\"status\":\"unsupported\"}" }
    nonisolated(unsafe) private static var constructing: NativeCapabilityScope? = nil
    nonisolated(unsafe) private static var owners: [String: NativeCapabilityScope] = [:]

    static func create<T>(_ owner: String, _ work: () -> T) -> T {
        let previous = constructing
        let scope = owners[owner] ?? NativeCapabilityScope(owner: owner, call: call)
        constructing = scope
        owners[owner] = scope
        defer { constructing = previous }
        return work()
    }

    private static func scope() -> NativeCapabilityScope {
        constructing ?? NativeCapabilityScope(owner: "", call: call)
    }

    static func filePicker() -> FilePicker {
        FilePicker(scope: scope())
    }

    static func windowControls() -> WindowControls {
        WindowControls(scope: scope())
    }

    static func dispose(_ owner: String) {
        owners.removeValue(forKey: owner)?.dispose()
    }
}
