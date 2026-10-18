@Setup
final class PageProgramSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        let identity = String(UInt64.random(in: 1...UInt64.max), radix: 16)
        var count = 0
        var adapters = BehaviorAdapters()
        adapters.http["pageprogram"] = BehaviorHTTPAdapter { request, _ in
            if try request.get("method").text == "POST", try request.get("path").text.hasSuffix("/increment") {
                count += 1
            }
            return .object(["identity": .string(identity), "count": .number(Double(count))])
        }
        return adapters
    }
}
