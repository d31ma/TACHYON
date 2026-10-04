@Service
final class CatalogService {
    var behaviors: BehaviorContext!
    var offset: Int = 0

    func page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) _ input: BehaviorJSON) throws -> BehaviorJSON {
        input.get("offset")
    }

    @OnStart
    func initialize() {
        behaviors = try? Behaviors.create(ApplicationSetup.self, partition: "public-catalog", signal: BehaviorSignal())
        offset = offsetOf(try? page(.object(["page": .number(1)])))
    }

    private func offsetOf(_ value: BehaviorJSON?) -> Int {
        guard case .number(let number)? = value else { return 0 }
        return Int(number)
    }
}
