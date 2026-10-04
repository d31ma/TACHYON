@Setup
final class UsersSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.http["users"] = BehaviorHTTPAdapter { request, execution in
            return .object(["name": request.get("body")])
        }
        return adapters
    }
}
