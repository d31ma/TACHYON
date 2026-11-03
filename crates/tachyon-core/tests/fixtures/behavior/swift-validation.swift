@Checked
final class AnnotatedInput {
    @NotEmpty
    @Length({"min":2,"max":8})
    var name: BehaviorJSON = .null
    @Required
    @Min(1)
    @Max(10)
    var amount: BehaviorJSON = .null
    @Length({"min":1,"max":2})
    var optional: BehaviorJSON = .null
    private init() {
        fatalError("must not instantiate")
    }
}
final class AnnotatedValidationService {
    var calls: Int = 0
    func accept(@Check(AnnotatedInput) _ input: BehaviorJSON) throws -> BehaviorJSON {
        calls += 1
        return input
    }
}
@Checked
final class AnnotatedPageInput {
    @Required
    @Min(1)
    var size: BehaviorJSON = .null
}
@CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
final class AnnotatedValidatedCache {
    @Needs
    let behaviors: BehaviorContext
    var calls: Int = 0
    @Cached
    func accept(@Check(AnnotatedInput) _ input: BehaviorJSON) async throws -> BehaviorJSON {
        calls += 1
        return input
    }
    func page(@Check(AnnotatedPageInput) @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) _ input: BehaviorJSON) throws -> BehaviorJSON {
        calls += 1
        return input
    }
    @WebCall({"client":"catalog","method":"GET","path":"/products"})
    func remote(@Check(AnnotatedInput) _ input: BehaviorJSON) async throws -> BehaviorJSON {
        calls += 1
        return .object([:])
    }
}
enum ValidationAnnotationTests {
    static func checkServiceValidation(_ valid: BehaviorJSON) throws {
        let service = AnnotatedValidationService()
        guard try Behaviors.validate(AnnotatedInput.self, valid) == valid, try service.accept(valid) == valid else { throw BehaviorError("VALIDATION_MUTATED") }
        let invalidInputs = [
            ("{\"name\":\"é\"}", "amount.notNull"),
            ("{\"name\":\"é\",\"amount\":0}", "amount.min"),
            ("{\"name\":\"é\",\"amount\":11}", "amount.max"),
            ("{\"name\":\"é\",\"amount\":\"1\"}", "amount.min"),
            ("{\"name\":\"x\",\"amount\":1}", "name.size"),
            ("{\"name\":\"é\",\"amount\":1,\"optional\":[]}", "optional.size"),
            ("[]", "$.object"),
        ]
        for (json, path) in invalidInputs {
            do {
                _ = try service.accept(BehaviorJSON.parse(json))
                throw BehaviorError("INVALID_ACCEPTED")
            } catch let error as BehaviorError where error.code == "validation" {
                guard error.path == path else { throw BehaviorError("WRONG_PATH") }
            }
        }
        for whitespace in ["", "\t\n\r ", "\u{85}\u{a0}\u{1680}\u{2000}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}"] {
            do {
                _ = try service.accept(.object(["name": .string(whitespace), "amount": .number(1)]))
                throw BehaviorError("BLANK_ACCEPTED")
            } catch let error as BehaviorError where error.code == "validation" && error.path == "name.notBlank" { }
        }
        guard service.calls == 1 else { throw BehaviorError("INVALID_BODY_RAN") }
    }
    static func checkDirectValidation(_ valid: BehaviorJSON) throws {
        for invalid in [BehaviorJSON.number(.nan), .object(["large": .string(String(repeating: "x", count: 1048577))])] {
            do {
                _ = try Behaviors.validate(AnnotatedInput.self, invalid)
                throw BehaviorError("INVALID_JSON_ACCEPTED")
            } catch let error as BehaviorError where error.code == "validation" && error.path == "$.json" { }
        }
        _ = try Behaviors.validate(AnnotatedInput.self, BehaviorJSON.parse("{\"name\":\"é\",\"amount\":1,\"optional\":null}"))
        _ = try Behaviors.validate(AnnotatedInput.self, BehaviorJSON.parse("{\"name\":\"é\",\"amount\":1,\"optional\":{\"a\":true}}"))
        _ = try Behaviors.validate(AnnotatedInput.self, .object(["name": .string("\u{200b}"), "amount": .number(1)]))
        do {
            _ = try Behaviors.validate(AnnotatedConfig.self, valid)
            throw BehaviorError("UNKNOWN_SCHEMA_ACCEPTED")
        } catch let error as BehaviorError where error.code == "validation" && error.path == "$.schema" { }
    }
    static func checkCachedValidation() async throws {
        let signal = BehaviorSignal()
        let context = try Behaviors.create(AnnotatedConfig.self, partition: "tenant-validation", signal: signal)
        defer { context.dispose() }
        let cached = AnnotatedValidatedCache(behaviors: context)
        let authorizations = Config.store.authorizations
        let reads = Config.store.reads
        do {
            _ = try await cached.accept(.object([:]))
            throw BehaviorError("INVALID_CACHE_ACCEPTED")
        } catch let error as BehaviorError where error.code == "validation" { }
        guard cached.calls == 0, authorizations == Config.store.authorizations, reads == Config.store.reads else { throw BehaviorError("INVALID_CACHE_BODY_ADAPTER_RAN") }
        do {
            _ = try cached.page(.object(["size": .number(0)]))
            throw BehaviorError("INVALID_PAGE_ACCEPTED")
        } catch let error as BehaviorError where error.code == "validation" && error.path == "size.min" { }
        do {
            _ = try await cached.remote(.object([:]))
            throw BehaviorError("INVALID_HTTP_ACCEPTED")
        } catch let error as BehaviorError where error.code == "validation" { }
        guard cached.calls == 0, try cached.page(.object(["size": .number(2), "page": .number(3)])).get("offset") == .number(6) else { throw BehaviorError("VALIDATION_PAGE_HTTP_ORDER") }
    }
    static func run() async throws {
        let valid = try BehaviorJSON.parse("{\"name\":\"é\",\"amount\":1,\"extra\":true}")
        try checkServiceValidation(valid)
        try checkDirectValidation(valid)
        try await checkCachedValidation()
    }
}
