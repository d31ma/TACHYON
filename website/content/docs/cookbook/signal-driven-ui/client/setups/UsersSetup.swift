@Setup
final class UsersSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var saved: [BehaviorJSON] = []
        var adapters = BehaviorAdapters()
        adapters.http["users"] = BehaviorHTTPAdapter { request, _ in
            if try request.get("method").text == "GET" { return .array(saved) }
            let user = BehaviorJSON.object(["name": request.get("body")])
            saved.append(user)
            return user
        }
        return adapters
    }
}
