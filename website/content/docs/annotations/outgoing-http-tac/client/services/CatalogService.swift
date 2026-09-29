@Service
final class CatalogService {
    let catalog: CatalogClient = CatalogClient()
    var count: Int = 0

    func all() async throws -> BehaviorJSON { return try await catalog.all() }
}
