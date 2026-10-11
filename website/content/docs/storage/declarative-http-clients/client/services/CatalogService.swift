@Service
final class CatalogService {
    @Needs
    private let catalog: CatalogClient
    var products: [String] = []
    var error: String = ""

    func load() async {
        show(try? await catalog.all())
    }

    private func show(_ answer: BehaviorJSON?) {
        guard let items = try? answer?.items else { return showFailure() }
        products = items.compactMap { try? $0.text }
        error = ""
    }

    private func showFailure() {
        error = "Products could not be loaded. Please retry."
    }
}
