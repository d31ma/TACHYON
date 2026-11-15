@MainActor
final class AppleCallAudio {
    static let shared = AppleCallAudio()
    static let earpiece = "earpiece"
    static let speaker = "speaker"
    static let portPrefix = "port:"
    static let unchanged = "The audio session could not change."
    static let unknownDevice = "There is no such speaker (unknown)."
    static let notOnThisSystem = "Call audio runs only on iPhone and iPad."
    private var speakerChosen = false

    func answer(_ capability: String, _ request: [String: Any], _ reply: @escaping (String) -> Void) {
        #if os(iOS)
        switch capability {
        case "browserApi.media.callActive": reply(callActive(request["on"] as? Bool ?? false))
        case "browserApi.media.speakers": reply(AppleBrowserApis.success(speakers()))
        case "browserApi.media.chooseSpeaker": reply(chooseSpeaker(request["device"] as? String ?? ""))
        default: reply(AppleBrowserApis.failure(AppleBrowserApis.notAnswered))
        }
        #else
        reply(AppleBrowserApis.failure(Self.notOnThisSystem))
        #endif
    }

    #if os(iOS)
    private func callActive(_ on: Bool) -> String {
        do {
            try Self.change(AVAudioSession.sharedInstance(), on)
            return AppleBrowserApis.success(["on": on, "background": Self.background()])
        } catch {
            return AppleBrowserApis.failure(Self.unchanged)
        }
    }

    private static func change(_ session: AVAudioSession, _ on: Bool) throws {
        guard on else { try session.setActive(false, options: .notifyOthersOnDeactivation); return }
        try session.setCategory(.playAndRecord, mode: .voiceChat, options: [.allowBluetoothHFP, .allowBluetoothA2DP])
        try session.setActive(true)
    }

    private static func background() -> Bool {
        let modes = Bundle.main.object(forInfoDictionaryKey: "UIBackgroundModes") as? [String] ?? []
        return modes.contains("audio")
    }

    private func speakers() -> [[String: Any]] {
        let session = AVAudioSession.sharedInstance()
        let ports = (session.availableInputs ?? []).filter(Self.isHeadset)
        let used = Set(session.currentRoute.inputs.map(\.uid))
        let headsets = ports.map { ["id": Self.portPrefix + $0.uid, "label": $0.portName, "chosen": used.contains($0.uid)] as [String: Any] }
        return Self.builtIn(speakerChosen) + headsets
    }

    private static func builtIn(_ speakerChosen: Bool) -> [[String: Any]] {
        let speaker: [String: Any] = ["id": Self.speaker, "label": "Speaker", "chosen": speakerChosen]
        guard UIDevice.current.userInterfaceIdiom == .phone else { return [speaker] }
        return [["id": Self.earpiece, "label": "iPhone", "chosen": !speakerChosen], speaker]
    }

    private static func isHeadset(_ port: AVAudioSessionPortDescription) -> Bool {
        [.bluetoothHFP, .headsetMic, .usbAudio, .carAudio].contains(port.portType)
    }

    private func chooseSpeaker(_ device: String) -> String {
        do {
            try route(AVAudioSession.sharedInstance(), device)
            return AppleBrowserApis.success(["device": device])
        } catch {
            return AppleBrowserApis.failure(Self.unknownDevice)
        }
    }

    private func route(_ session: AVAudioSession, _ device: String) throws {
        speakerChosen = device == Self.speaker
        try session.overrideOutputAudioPort(speakerChosen ? .speaker : .none)
        if speakerChosen { return }
        try session.setPreferredInput(try Self.port(session, device))
    }

    private static func port(_ session: AVAudioSession, _ device: String) throws -> AVAudioSessionPortDescription? {
        let wanted = device == Self.earpiece ? nil : String(device.dropFirst(Self.portPrefix.count))
        let inputs = session.availableInputs ?? []
        if wanted == nil { return inputs.first { $0.portType == .builtInMic } }
        guard let found = inputs.first(where: { $0.uid == wanted }) else { throw CocoaError(.featureUnsupported) }
        return found
    }
    #endif
}
