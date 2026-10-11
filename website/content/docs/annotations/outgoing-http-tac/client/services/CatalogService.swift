@Service
final class CatalogService {
    @Needs
    private let catalog: CatalogClient
    var count: Int = 0

    func load() async throws {
        count = CatalogService.countItems(try await catalog.all())
    }

    private static func countItems(_ answer: BehaviorJSON) -> Int {
        guard case .array(let items) = answer.get("items") else { return 0 }
        return items.count
    }
}
