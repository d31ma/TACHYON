@Service
final class CatalogService {
    @Needs
    var productConverter: ProductConverter!
    var result: String = ""

    func load() throws {
        let detail = Detail(name: "detail", secret: "detail-private")
        let product = try productConverter.copy(Product(id: 7, name: "Anvil", detail: detail, items: [detail], secret: "product-private"))
        result = text(try productConverter.map(product))
    }

    private func text(_ summary: Summary) -> String {
        return "\(summary.kind) \(summary.id): \(summary.name), \(summary.detail.name), \(summary.items.count)"
    }
}
