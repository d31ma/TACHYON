final class WindowControls {
    private let scope: NativeCapabilityScope

    init(scope: NativeCapabilityScope) {
        self.scope = scope
    }

    func granted(_ control: String) -> Bool {
        guard scope.isActive,
              let request = try? JSONSerialization.data(withJSONObject: ["owner": scope.owner, "control": control]),
              let result = try? JSONSerialization.jsonObject(with: Data(scope.invoke("window.granted", String(decoding: request, as: UTF8.self)).utf8)) as? [String: Any] else { return false }
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
        guard granted(control) else { throw NativeCapabilityFailure.denied }
        var request = values
        request["owner"] = scope.owner
        guard let data = try? JSONSerialization.data(withJSONObject: request),
              let result = try? JSONSerialization.jsonObject(with: Data(scope.invoke("window." + control, String(decoding: data, as: UTF8.self)).utf8)) as? [String: Any],
              result["ok"] as? Bool == true else { throw NativeCapabilityFailure.unavailable }
    }
}
