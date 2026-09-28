final class Product {}
final class ApplicationProperties {
    init() {
        fatalError("Property binding must not instantiate DTOs")
    }
}
final class InvalidProperties {}
final class GuardedConfig {
    static var calls = 0
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        calls += 1
        return Config.configure(execution)
    }
}
final class MissingConfig {
    static var calls = 0
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        calls += 1
        return Config.configure(execution)
    }
}
final class MemoryCache {
    var values: [String: BehaviorJSON] = [:]
    var authorizations = 0
    var allowed = true
    var includeCache = true
    var reads = 0
    var writes = 0
    var deletes = 0
    var lastCall: BehaviorJSON = .null
    var entityAllowed = true
    var includeEntityAuthorization = true
    var entityAuthorizations = 0
    var entityExecutions = 0
    var queryResult: BehaviorJSON? = nil
    var lastQuery: BehaviorJSON = .null
    func adapter() -> BehaviorCacheAdapter {
        BehaviorCacheAdapter(get: { key, _ in
            self.reads += 1
            return self.values[key] ?? .object(["hit": .bool(false)])
        }, set: { key, value, policy, expiry, _ in
            self.writes += 1
            let maximum = try policy.get("maxEntries").integer
            guard self.values[key] != nil || self.values.count < maximum else { throw BehaviorError("QUOTA") }
            self.values[key] = .object(["hit": .bool(true), "value": value, "expiresAt": .number(Double(expiry))])
        }, delete: { key, _ in
            self.deletes += 1
            self.values.removeValue(forKey: key)
        })
    }
}
final class Config {
    static let store = MemoryCache()
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        if store.includeCache { adapters.caches["products"] = store.adapter() }
        adapters.authorize = { call, execution in
            store.authorizations += 1
            store.lastCall = call
            guard execution.partition == "tenant-one", call.has("keyArguments") else { throw BehaviorError("PARTITION") }
            return store.allowed
        }
        adapters.http["catalog"] = BehaviorHTTPAdapter { request, execution in
            guard request.get("redirect") == .string("error"), request.get("credentials") == .string("omit"), request.get("maxResponseBytes") == .number(1_048_576) else { throw BehaviorError("TRANSPORT_BOUNDS") }
            return .object(["transport": .bool(true)])
        }
        adapters.entities["data"] = BehaviorEntityAdapter { command, execution in
            store.entityExecutions += 1
            guard command.get("table") == .string("products"), command.get("idField") == .string("id") else { throw BehaviorError("MAPPING") }
            guard command.get("operation") == .string("query") else { return command }
            store.lastQuery = command
            if let result = store.queryResult { return result }
            switch command.get("query").get("kind") {
            case .string("find"):
                return .array([.object(["id": .number(1)])])
            case .string("count"):
                return .number(1)
            default:
                return .bool(true)
            }
        }
        if store.includeEntityAuthorization {
            adapters.authorizeEntity = { command, execution in
                store.entityAuthorizations += 1
                try execution.check()
                guard command.get("operation") == .string("query") else { throw BehaviorError("QUERY_AUTH_COMMAND") }
                return store.entityAllowed
            }
        }
        return adapters
    }
}
enum NativeBehaviorTests {
    static var loadCalls = 0
    static var mutationCalls = 0
    static func check(_ condition: Bool, _ message: String) throws {
        guard condition else { throw BehaviorError(message) }
    }
    static func expectError(_ code: String, _ body: () async throws -> Void) async throws {
        do {
            try await body()
        } catch let error as BehaviorError {
            try check(error.code == code, "WRONG_CODE_" + error.code)
            return
        }
        throw BehaviorError("EXPECTED_" + code)
    }
    static func expectRejectedProperties(_ type: Any.Type, _ supplied: [String: String]) async throws {
        try await expectError("configuration-properties") { _ = try Behaviors.properties(type, supplied) }
    }
    static func load() async throws -> BehaviorJSON {
        loadCalls += 1
        return .object(["result": .number(1)])
    }
    static func countMutation(_ result: BehaviorJSON) -> BehaviorJSON {
        mutationCalls += 1
        return result
    }
    static func queryMetadata(_ operation: String) -> BehaviorJSON {
        let entity = BehaviorJSON.object(["name": .string("Product"), "adapter": .string("data"), "id": .string("id"), "table": .string("products")])
        let nameEquals = BehaviorJSON.object(["field": .string("name"), "operator": .string("eq"), "argument": .number(0)])
        let idIn = BehaviorJSON.object(["field": .string("id"), "operator": .string("in"), "argument": .number(1)])
        let nameIsNotNull = BehaviorJSON.object(["field": .string("name"), "operator": .string("isNotNull")])
        var pageArgument = BehaviorJSON.null
        if operation == "find" { pageArgument = .number(2) }
        let orderBy = BehaviorJSON.array([.object(["field": .string("id"), "direction": .string("desc")])])
        return .object(["entity": entity, "fields": .array([.string("id"), .string("name")]), "defaultSize": .number(20), "maxSize": .number(100), "operation": .string(operation), "where": .array([nameEquals, idIn, nameIsNotNull]), "pageArgument": pageArgument, "orderBy": orderBy])
    }
    static func paginateNested(_ context: BehaviorContext, depth: Int) throws {
        try context.paginate {
            if depth > 0 { try paginateNested(context, depth: depth - 1) }
        }
    }
    static func checkPropertyBinding() async throws {
        var supplied = ["TAC_APP_LABEL": "catalog", "TAC_APP_ENABLED": "true", "TAC_APP_WORKERS": "8", "TAC_APP_RATIO": "2.5e0", "TAC_OTHER_VALUE": "ignored"]
        let bound = try Behaviors.properties(ApplicationProperties.self, supplied)
        try check(bound.get("label") == .string("catalog") && bound.get("enabled") == .bool(true) && bound.get("workers") == .number(8) && bound.get("ratio") == .number(2.5), "CONFIGURATION_PROPERTIES")
        supplied["TAC_APP_WORKERS"] = "16"
        try check(bound.get("workers") == .number(8), "PROPERTY_SNAPSHOT")
        let defaults = try Behaviors.properties(ApplicationProperties.self, ["TAC_APP_LABEL": "catalog"])
        let namespaced = try Behaviors.properties(ApplicationProperties.self, try BehaviorJSON.parse("{\"tac\":{\"app\":{\"label\":\"catalog\",\"workers\":8,\"enabled\":true}}}"))
        try check(namespaced.get("workers") == .number(8) && namespaced.get("enabled") == .bool(true), "NAMESPACED_BINDING")
        try check(defaults.get("workers") == .number(4) && defaults.get("enabled") == .bool(false), "PROPERTY_DEFAULTS")
        try await expectRejectedProperties(ApplicationProperties.self, [:])
        try await expectRejectedProperties(InvalidProperties.self, [:])
        try await checkRejectedPropertyValues(supplied)
    }
    static func checkRejectedPropertyValues(_ original: [String: String]) async throws {
        var supplied = original
        for invalid in ["01", "+1", "1.0", "1e0", " 1", "1\n", "9007199254740992", "0", "65"] {
            supplied["TAC_APP_WORKERS"] = invalid
            try await expectRejectedProperties(ApplicationProperties.self, supplied)
        }
        supplied["TAC_APP_WORKERS"] = "8"
        supplied["TAC_APP_ENABLED"] = "True"
        try await expectRejectedProperties(ApplicationProperties.self, supplied)
        supplied["TAC_APP_ENABLED"] = "true"
        supplied["TAC_APP_TYPO"] = "hidden"
        try await expectRejectedProperties(ApplicationProperties.self, supplied)
        supplied.removeValue(forKey: "TAC_APP_TYPO")
        supplied["YON_APP_LABEL"] = "hidden"
        try await expectRejectedProperties(ApplicationProperties.self, supplied)
        supplied.removeValue(forKey: "YON_APP_LABEL")
        supplied["TAC_APP_LABEL"] = String(repeating: "x", count: 4097)
        try await expectRejectedProperties(ApplicationProperties.self, supplied)
    }
    static func checkConfigurationSelection(_ signal: BehaviorSignal) async throws {
        try await expectError("inactive-configuration") { _ = try Behaviors.create(GuardedConfig.self, partition: "tenant-one", signal: signal) }
        try await expectError("inactive-configuration") { _ = try Behaviors.create(GuardedConfig.self, partition: "tenant-one", signal: signal, profiles: ["testing"]) }
        try check(GuardedConfig.calls == 0, "INACTIVE_FACTORY")
        var selectedProfiles = ["testing"]
        var selectedProperties = ["TAC_CACHE_ENABLED": "true"]
        let selectedContext = try Behaviors.create(GuardedConfig.self, partition: "tenant-one", signal: signal, profiles: selectedProfiles, properties: selectedProperties)
        defer { selectedContext.dispose() }
        try check(GuardedConfig.calls == 1, "COMBINED_CONDITIONS")
        selectedProfiles[0] = "disabled"
        selectedProperties["TAC_CACHE_ENABLED"] = "false"
        try check(selectedContext.paginate { 42 } == 42 && GuardedConfig.calls == 1, "CREATION_SNAPSHOT")
        let nestedContext = try Behaviors.create(GuardedConfig.self, partition: "tenant-one", signal: signal, profiles: ["testing"], properties: try BehaviorJSON.parse("{\"tac\":{\"cache\":{\"enabled\":true}}}"))
        nestedContext.dispose()
        let missing = try Behaviors.create(MissingConfig.self, partition: "tenant-one", signal: signal)
        defer { missing.dispose() }
        try check(MissingConfig.calls == 1, "MATCH_IF_MISSING")
        try await expectError("configuration") { _ = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal, profiles: ["dev", "dev"]) }
        try await expectError("configuration") { _ = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal, properties: ["YON_WRONG": "hidden"]) }
    }
    static func checkCachedCalls(_ context: BehaviorContext, _ argument: BehaviorJSON, _ normalized: BehaviorJSON) async throws {
        _ = try await context.invoke("Catalog", "list", arguments: [normalized], body: load)
        _ = try await context.invoke("Catalog", "list", arguments: [normalized], body: load)
        try check(loadCalls == 1 && Config.store.authorizations == 2, "CACHE")
        _ = try await context.evict("Catalog", "list", arguments: [argument])
        _ = try await context.invoke("Catalog", "list", arguments: [normalized], body: load)
        try check(loadCalls == 2 && Config.store.authorizations == 4, "EVICTION")
        Config.store.allowed = false
        try await expectError("FORBIDDEN") { _ = try await context.invoke("Catalog", "list", arguments: [normalized], body: load) }
        Config.store.allowed = true
        let response = try await context.invoke("Catalog", "remote", arguments: []) { .object([:]) }
        try check(response.get("transport") == .bool(true), "HTTP")
    }
    static func checkRepository(_ context: BehaviorContext, _ argument: BehaviorJSON) async throws {
        let repository = try context.repository(Product.self)
        let found = try await repository.find(argument)
        try check(found.get("page").get("offset") == .number(2), "REPOSITORY_PAGE")
        let saved = try await repository.save(.object(["id": .number(4)]))
        try check(saved.get("operation") == .string("save"), "REPOSITORY_SAVE")
        let deleted = try await repository.delete(.number(4))
        try check(deleted.get("id") == .number(4), "REPOSITORY_DELETE")
    }
    static func checkStrictInput(_ context: BehaviorContext, _ pagePolicy: BehaviorJSON) async throws {
        try await expectError("PAGE_SORT") { _ = try BehaviorPage.normalize(.object(["sort": .array([.string("unknown")])]), policy: pagePolicy) }
        try await expectError("JSON_DUPLICATE") { _ = try BehaviorJSON.parse("{\"x\":1,\"x\":2}") }
        try await expectError("JSON_NUMBER") { _ = try BehaviorJSON.number(.infinity).encoded() }
        try await expectError("HTTP_HEADER") { _ = try await context.invoke("Catalog", "remote", arguments: []) { .object(["headers": .object(["x": .string("bad\r\nvalue")])]) } }
        try await expectError("HTTP_HEADER") { _ = try await context.invoke("Catalog", "remote", arguments: []) { .object(["headers": .object(["Cookie": .string("session=invalid")])]) } }
        try await expectError("HTTP_BODY") { _ = try await context.invoke("Catalog", "remote", arguments: []) { .object(["body": .null]) } }
        try await expectError("JSON_TYPE") { _ = try BehaviorJSON.from(Foundation.Date()) }
        try check(BehaviorJSON.parse("1.0").encoded() == BehaviorJSON.parse("1").encoded(), "NUMERIC_KEYS")
    }
    static func checkOperationLimit(_ context: BehaviorContext) async throws {
        try await expectError("OPERATION_LIMIT") { try paginateNested(context, depth: 64) }
    }
    static func checkCachePut(_ context: BehaviorContext, _ argument: BehaviorJSON, _ normalized: BehaviorJSON, _ replacement: BehaviorJSON) async throws {
        let readsBefore = Config.store.reads
        let writesBefore = Config.store.writes
        _ = try await context.invoke("Catalog", "update", arguments: [replacement, argument]) { countMutation(replacement) }
        try check(mutationCalls == 1 && Config.store.reads == readsBefore && Config.store.writes == writesBefore + 1, "PUT_NO_READ")
        let lastCall = Config.store.lastCall
        try check(lastCall.get("operation") == .string("cache:put") && lastCall.get("method") == .string("list") && lastCall.get("keyArguments").items[0].get("offset") == .number(2), "MUTATION_TARGET")
        let refreshed = try await context.invoke("Catalog", "list", arguments: [normalized], body: load)
        try check(refreshed == replacement && loadCalls == 2, "PUT_KEY")
    }
    static func checkCacheEvict(_ context: BehaviorContext, _ argument: BehaviorJSON, _ normalized: BehaviorJSON) async throws {
        let deletesBefore = Config.store.deletes
        _ = try await context.invoke("Catalog", "remove", arguments: [argument]) { countMutation(.null) }
        try check(Config.store.deletes == deletesBefore + 1 && Config.store.lastCall.get("operation") == .string("cache:evict"), "EVICT_AFTER_MUTATION")
        _ = try await context.invoke("Catalog", "list", arguments: [normalized], body: load)
        try check(loadCalls == 3, "EVICT_KEY")
    }
    static func checkRejectedMutations(_ context: BehaviorContext, _ signal: BehaviorSignal, _ argument: BehaviorJSON, _ replacement: BehaviorJSON) async throws {
        let writesBefore = Config.store.writes
        let deletesBefore = Config.store.deletes
        Config.store.allowed = false
        try await expectError("FORBIDDEN") { _ = try await context.invoke("Catalog", "update", arguments: [replacement, argument]) { countMutation(replacement) } }
        Config.store.allowed = true
        try check(mutationCalls == 2 && Config.store.writes == writesBefore, "DENIED_MUTATION")
        try await expectError("CACHE_ARGUMENT") { _ = try await context.invoke("Catalog", "update", arguments: [replacement]) { countMutation(replacement) } }
        Config.store.includeCache = false
        let missingAdapter = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal)
        Config.store.includeCache = true
        try await expectError("CACHE_MISSING") { _ = try await missingAdapter.invoke("Catalog", "update", arguments: [replacement, argument]) { countMutation(replacement) } }
        missingAdapter.dispose()
        try check(mutationCalls == 2, "INVALID_MUTATION_PREREQUISITES")
        try await expectError("APPLICATION_FAILED") { _ = try await context.invoke("Catalog", "remove", arguments: [argument]) { throw BehaviorError("APPLICATION_FAILED") } }
        try check(Config.store.deletes == deletesBefore, "FAILED_MUTATION")
        try await expectError("JSON_LIMIT") { _ = try await context.invoke("Catalog", "update", arguments: [replacement, argument]) { .string(String(repeating: "x", count: 65537)) } }
        try check(Config.store.writes == writesBefore, "OVERSIZE_MUTATION")
    }
    static func checkHttpCachePut(_ context: BehaviorContext, _ argument: BehaviorJSON, _ normalized: BehaviorJSON) async throws {
        _ = try await context.invoke("Catalog", "updateRemote", arguments: [argument]) { .object([:]) }
        let exchanged = try await context.invoke("Catalog", "list", arguments: [normalized], body: load)
        try check(exchanged.get("transport") == .bool(true), "PUT_HTTP_RESULT")
    }
    static func checkInterruptedMutations(_ signal: BehaviorSignal, _ argument: BehaviorJSON, _ replacement: BehaviorJSON) async throws {
        let cancelledMutation = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal)
        let writesBefore = Config.store.writes
        try await expectError("CANCELLED") {
            _ = try await cancelledMutation.invoke("Catalog", "update", arguments: [replacement, argument]) {
                cancelledMutation.dispose()
                return replacement
            }
        }
        try check(Config.store.writes == writesBefore, "CANCELLED_MUTATION")
        let timedMutation = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal, timeoutMs: 10)
        try await expectError("DEADLINE") {
            _ = try await timedMutation.invoke("Catalog", "update", arguments: [replacement, argument]) {
                try await Task.sleep(nanoseconds: 30_000_000)
                return replacement
            }
        }
        timedMutation.dispose()
        try check(Config.store.writes == writesBefore, "EXPIRED_MUTATION")
    }
    static func checkQueries(_ context: BehaviorContext, _ signal: BehaviorSignal) async throws {
        let queryArguments: [BehaviorJSON] = [.string("tools"), .array([.number(1), .number(2)]), .object(["page": .number(1), "size": .number(2)])]
        try await checkAuthorizedQueries(context, queryArguments)
        try await checkRejectedQueries(context, signal, queryArguments)
        try await checkInvalidQueryResults(context, queryArguments)
    }
    static func checkAuthorizedQueries(_ context: BehaviorContext, _ queryArguments: [BehaviorJSON]) async throws {
        let executionsBefore = Config.store.entityExecutions
        let authorizationsBefore = Config.store.entityAuthorizations
        let rows = try await context.query(queryMetadata("find"), arguments: queryArguments)
        try check(rows.items.count == 1 && Config.store.entityExecutions == executionsBefore + 1 && Config.store.entityAuthorizations == authorizationsBefore + 1, "QUERY_AUTH_EXECUTION")
        let query = Config.store.lastQuery.get("query")
        try check(query.get("page").get("offset") == .number(2) && query.get("orderBy").items[0].get("direction") == .string("desc"), "QUERY_PAGE_ORDER")
        try check(query.get("where").items[1].get("value").items.count == 2 && !query.get("where").items[2].has("value"), "QUERY_BINDINGS")
        let counted = try await context.query(queryMetadata("count"), arguments: queryArguments)
        try check(counted == .number(1) && !Config.store.lastQuery.get("query").has("page"), "QUERY_COUNT")
        let exists = try await context.query(queryMetadata("exists"), arguments: queryArguments)
        try check(exists == .bool(true), "QUERY_EXISTS")
    }
    static func checkRejectedQueries(_ context: BehaviorContext, _ signal: BehaviorSignal, _ queryArguments: [BehaviorJSON]) async throws {
        let executionsBefore = Config.store.entityExecutions
        Config.store.entityAllowed = false
        try await expectError("FORBIDDEN") { _ = try await context.query(queryMetadata("find"), arguments: queryArguments) }
        Config.store.entityAllowed = true
        Config.store.includeEntityAuthorization = false
        let unauthenticatedQuery = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal)
        Config.store.includeEntityAuthorization = true
        try await expectError("FORBIDDEN") { _ = try await unauthenticatedQuery.query(queryMetadata("find"), arguments: queryArguments) }
        unauthenticatedQuery.dispose()
        try check(Config.store.entityExecutions == executionsBefore, "QUERY_DENIED_NO_EXECUTION")
        try await expectError("QUERY_BINDING") { _ = try await context.query(queryMetadata("find"), arguments: [.object([:]), queryArguments[1], queryArguments[2]]) }
        let tooManyValues = BehaviorJSON.array(Array(repeating: .number(1), count: 101))
        try await expectError("QUERY_BINDING") { _ = try await context.query(queryMetadata("find"), arguments: [queryArguments[0], tooManyValues, queryArguments[2]]) }
        try await expectError("JSON_LIMIT") { _ = try await context.query(queryMetadata("find"), arguments: [.string(String(repeating: "x", count: 65536)), queryArguments[1], queryArguments[2]]) }
        try await expectError("QUERY_ORDER") { _ = try await context.query(queryMetadata("find"), arguments: [queryArguments[0], queryArguments[1], .object(["sort": .array([.string("id")])])]) }
        try check(Config.store.entityExecutions == executionsBefore, "QUERY_INVALID_NO_EXECUTION")
    }
    static func checkInvalidQueryResults(_ context: BehaviorContext, _ queryArguments: [BehaviorJSON]) async throws {
        Config.store.queryResult = .array([.object([:]), .object([:]), .object([:])])
        try await expectError("QUERY_RESULT") { _ = try await context.query(queryMetadata("find"), arguments: queryArguments) }
        Config.store.queryResult = .array([.number(1)])
        try await expectError("JSON_OBJECT") { _ = try await context.query(queryMetadata("find"), arguments: queryArguments) }
        Config.store.queryResult = .number(-1)
        try await expectError("QUERY_RESULT") { _ = try await context.query(queryMetadata("count"), arguments: queryArguments) }
        Config.store.queryResult = .number(1)
        try await expectError("JSON_BOOLEAN") { _ = try await context.query(queryMetadata("exists"), arguments: queryArguments) }
        Config.store.queryResult = nil
    }
    static func checkDeadlineAndCancellation(_ context: BehaviorContext, _ signal: BehaviorSignal, _ normalized: BehaviorJSON) async throws {
        let short = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal, timeoutMs: 10)
        try await expectError("DEADLINE") {
            _ = try await short.invoke("Catalog", "remote", arguments: []) {
                try await Task.sleep(nanoseconds: 30_000_000)
                return .object([:])
            }
        }
        short.dispose()
        context.dispose()
        try await expectError("CANCELLED") { _ = try await context.invoke("Catalog", "list", arguments: [normalized], body: load) }
        let owned = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal)
        signal.abort()
        try await expectError("CANCELLED") { _ = try await owned.invoke("Catalog", "list", arguments: [normalized], body: load) }
    }
    static func run() async throws {
        let signal = BehaviorSignal()
        let context = try Behaviors.create(Config.self, partition: "tenant-one", signal: signal)
        defer { context.dispose() }
        try await checkPropertyBinding()
        try await checkConfigurationSelection(signal)
        let argument = try BehaviorJSON.parse("{\"page\":1,\"size\":2,\"sort\":[\"id\"]}")
        let pagePolicy = try BehaviorJSON.parse("{\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"],\"argument\":0}")
        let normalized = try BehaviorPage.normalize(argument, policy: pagePolicy)
        try check(normalized.get("offset") == .number(2) && normalized.get("sort") == .array([.string("id:asc")]), "PAGE")
        try await checkCachedCalls(context, argument, normalized)
        try await checkRepository(context, argument)
        try await checkStrictInput(context, pagePolicy)
        try await checkOperationLimit(context)
        let replacement = BehaviorJSON.object(["updated": .bool(true)])
        try await checkCachePut(context, argument, normalized, replacement)
        try await checkCacheEvict(context, argument, normalized)
        try await checkRejectedMutations(context, signal, argument, replacement)
        try await checkHttpCachePut(context, argument, normalized)
        try await checkInterruptedMutations(signal, argument, replacement)
        try await checkQueries(context, signal)
        try await checkDeadlineAndCancellation(context, signal, normalized)
        print("Swift behavior runtime: cache mutations, authorized queries, pagination, HTTP, persistence, strict JSON, deadlines, disposal passed")
    }
}
