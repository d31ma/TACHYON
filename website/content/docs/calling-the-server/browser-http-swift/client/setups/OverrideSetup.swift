@Setup
final class OverrideSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.http["override"] = BehaviorHTTPAdapter { _, _ in
            .object(["message": .string("Custom adapter wins")])
        }
        return adapters
    }
}
