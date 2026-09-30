enum NativeCapabilities {
    nonisolated(unsafe) static var call: (String, String) -> String = { _, _ in "{\"status\":\"unsupported\"}" }
    nonisolated(unsafe) private static var constructing: String? = nil
    nonisolated(unsafe) private static var owners: Set<String> = []

    static func create<T>(_ owner: String, _ work: () -> T) -> T {
        let previous = constructing
        constructing = owner
        owners.insert(owner)
        defer { constructing = previous }
        return work()
    }

    static func active(_ owner: String) -> Bool {
        Thread.isMainThread && owners.contains(owner)
    }

    static func filePicker() -> FilePicker {
        FilePicker(owner: constructing ?? "", call: call)
    }

    static func windowControls() -> WindowControls {
        WindowControls(owner: constructing ?? "", call: call)
    }

    static func dispose(_ owner: String) {
        owners.remove(owner)
        guard let data = try? JSONSerialization.data(withJSONObject: ["owner": owner]),
              let request = String(data: data, encoding: .utf8) else { return }
        _ = call("filePicker.dispose", request)
    }
}
