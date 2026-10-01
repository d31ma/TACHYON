final class NativeCapabilityScope {
    let owner: String
    private var active: Bool
    private let call: (String, String) -> String

    init(owner: String, call: @escaping (String, String) -> String) {
        self.owner = owner
        self.active = !owner.isEmpty
        self.call = call
    }

    var isActive: Bool { Thread.isMainThread && active }

    func invoke(_ operation: String, _ payload: String) -> String {
        guard isActive else { return "{\"status\":\"disposed\"}" }
        return call(operation, payload)
    }

    func dispose() {
        guard active else { return }
        active = false
        guard let data = try? JSONSerialization.data(withJSONObject: ["owner": owner]),
              let request = String(data: data, encoding: .utf8) else { return }
        _ = call("filePicker.dispose", request)
    }
}
