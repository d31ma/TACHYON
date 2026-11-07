@TestSetup
final class HelloTestSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.http["greeting"] = BehaviorHTTPAdapter { _, _ in
            .object(["message": .string("Hi")])
        }
        return adapters
    }
}
