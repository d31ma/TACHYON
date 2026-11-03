import Foundation

@main
struct HeldClientsCaller {
    nonisolated(unsafe) static var signals: [String] = []
    static let chatOwner = "00000000-0000-4000-8000-000000000001"
    static let streamOwner = "00000000-0000-4000-8000-000000000002"

    static func ask(_ owner: String, _ service: String, _ rest: String) -> String {
        tacNativeInvoke("{\"route\":\"/\",\"service\":\"" + service + "\",\"owner\":\"" + owner + "\"," + rest + "}")
    }

    static func described(_ answer: String) -> [String: Any] {
        let parsed = try? JSONSerialization.jsonObject(with: Data(answer.utf8))
        return ((parsed as? [String: Any])?["value"] as? [String: Any]) ?? [:]
    }

    static func first(_ value: Any?, _ key: String) -> String {
        guard let list = value as? [Any], let first = list.first else { return "" }
        if let text = first as? String { return text }
        return ((first as? [String: Any])?[key] as? String) ?? ""
    }

    static func main() {
        TacBridge.emit = { signals.append($0) }
        let chatInit = ask(chatOwner, "chat", "\"op\":\"init\"")
        print("chat init " + chatInit)
        let chat = described(chatInit)
        let heard = first(chat["listeners"], "key")
        print("chat event " + ask(chatOwner, "chat", "\"op\":\"event\",\"name\":\"" + heard + "\",\"value\":{\"text\":\"from the page\"}"))
        print("chat field " + ask(chatOwner, "chat", "\"op\":\"get\",\"name\":\"" + first(chat["fields"], "") + "\""))
        print("signals before send " + String(signals.count))
        let method = (chat["methods"] as? [String])?.first { $0.lowercased() == "sayhello" } ?? ""
        print("chat send " + ask(chatOwner, "chat", "\"op\":\"call\",\"name\":\"" + method + "\",\"args\":[]"))
        for signal in signals { print("chat signal " + signal) }
        let streamInit = ask(streamOwner, "stream", "\"op\":\"init\"")
        print("stream init " + streamInit)
        let stream = described(streamInit)
        let tick = first(stream["listeners"], "key")
        print("stream event " + ask(streamOwner, "stream", "\"op\":\"event\",\"name\":\"" + tick + "\",\"value\":{\"sku\":\"A\",\"level\":7}"))
        print("stream field " + ask(streamOwner, "stream", "\"op\":\"get\",\"name\":\"" + first(stream["fields"], "") + "\""))
        print("dispose " + tacNativeInvoke("{\"op\":\"host-dispose\"}"))
    }
}
