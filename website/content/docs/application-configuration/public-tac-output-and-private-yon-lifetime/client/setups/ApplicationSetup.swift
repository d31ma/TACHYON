@Setup
final class ApplicationSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.http["application"] = BehaviorHTTPAdapter { request, execution in
            return try BehaviorJSON.parse("{\"tac\":{\"api\":{\"url\":\"/api\"}}}")
        }
        return adapters
    }
}
