@Service
final class CatalogService {
    var behaviors: BehaviorContext!
    var offset: Int = 0

    func page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) _ input: BehaviorJSON) async throws -> BehaviorJSON {
        input.get("offset")
    }

    @OnStart
    func initialize() async throws {
        behaviors = try Behaviors.create(ApplicationSetup.self, partition: "public-catalog", signal: BehaviorSignal())
        if case .number(let first) = try await page(.object(["page": .number(1)])) {
            offset = Int(first)
        }
    }
}
