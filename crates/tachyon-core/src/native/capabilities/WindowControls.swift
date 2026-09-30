final class WindowControls {
    private let owner: String
    private let call: (String, String) -> String

    init(owner: String, call: @escaping (String, String) -> String) {
        self.owner = owner
        self.call = call
    }

    func granted(_ control: String) -> Bool {
        guard NativeCapabilities.active(owner),
              let request = try? JSONSerialization.data(withJSONObject: ["control": control]),
              let result = try? JSONSerialization.jsonObject(with: Data(call("window.granted", String(decoding: request, as: UTF8.self)).utf8)) as? [String: Any] else { return false }
        return result["granted"] as? Bool ?? false
    }

    func minimize() throws { try invoke("minimize") }
    func maximize() throws { try invoke("maximize") }
    func fullscreen() throws { try invoke("fullscreen") }
    func close() throws { try invoke("close") }
    func resize(_ width: Int, _ height: Int) throws {
        guard (64...16384).contains(width), (64...16384).contains(height) else { throw NativeCapabilityFailure.invalid }
        try invoke("resize", ["width": width, "height": height])
    }
    func setTitle(_ title: String) throws { try invoke("title", ["title": title]) }

    private func invoke(_ control: String, _ values: [String: Any] = [:]) throws {
        guard granted(control), let data = try? JSONSerialization.data(withJSONObject: values) else {
            throw NativeCapabilityFailure.denied
        }
        _ = call("window." + control, String(decoding: data, as: UTF8.self))
    }
}
