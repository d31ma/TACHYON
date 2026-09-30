@Service
final class CatalogService {
    private let products: ProductsClient = ProductsClient()
    var count: Int = 0

    func load() async throws {
        count = try await products.list().get("items").items.count
    }

    func add() async throws {
        _ = try await products.save(product: .object(["name": .string("anvil")]))
    }
}
