@Service
@HTTPClient
final class CatalogService {
    let behaviors: BehaviorContext = try! Behaviors.create(
        CatalogSetup.self, partition: "catalog", signal: BehaviorSignal())
    var count: Int = 0

    @GetRequest("/api/products")
    func all() async throws -> BehaviorJSON { return .object([:]) }
}
