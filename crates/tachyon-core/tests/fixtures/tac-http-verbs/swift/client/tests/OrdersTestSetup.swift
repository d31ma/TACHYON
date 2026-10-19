@TestSetup
final class OrdersTestSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.http["orders"] = BehaviorHTTPAdapter { request, _ in
            let headers = request.get("headers")
            return .object([
                "method": request.get("method"),
                "path": request.get("path"),
                "header": headers.has("X-Tenant") ? headers.get("X-Tenant") : headers.get("X-Reason"),
                "item": request.has("body") ? request.get("body").get("item") : .string("none"),
            ])
        }
        return adapters
    }
}
