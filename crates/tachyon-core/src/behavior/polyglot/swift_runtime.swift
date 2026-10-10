struct BehaviorError: Error, CustomStringConvertible {
    let code: String
    let path: String?
    init(_ code: String, path: String? = nil) { self.code = code; self.path = path.flatMap { $0.utf8.count <= 128 ? $0 : nil } }
    var description: String { "Behavior operation failed: " + code }
}
indirect enum BehaviorJSON: Equatable {
    case null, bool(Bool), number(Double), string(String), array([BehaviorJSON]), object([String: BehaviorJSON])
    static func parse(_ source: String) throws -> BehaviorJSON {
        guard source.utf8.count <= 1_048_576 else { throw BehaviorError("JSON_LIMIT") }
        var reader = BehaviorJSONReader(bytes: Array(source.utf8)); let value = try reader.read(0)
        reader.space(); guard reader.index == reader.bytes.count else { throw BehaviorError("JSON_INVALID") }; return value
    }
    static func from(_ value: Any) throws -> BehaviorJSON {
        if let json = value as? BehaviorJSON { _ = try json.encoded(); return json }
        switch value {
        case let value as Bool: return .bool(value)
        case let value as String: return .string(value)
        case let value as Int: return try checkedNumber(Double(value))
        case let value as Int8: return try checkedNumber(Double(value))
        case let value as Int16: return try checkedNumber(Double(value))
        case let value as Int32: return try checkedNumber(Double(value))
        case let value as Int64: return try checkedNumber(Double(value))
        case let value as UInt: return try checkedNumber(Double(value))
        case let value as UInt8: return try checkedNumber(Double(value))
        case let value as UInt16: return try checkedNumber(Double(value))
        case let value as UInt32: return try checkedNumber(Double(value))
        case let value as UInt64: return try checkedNumber(Double(value))
        case let value as Double: return try checkedNumber(value)
        case let value as Float: return try checkedNumber(Double(value))
        default: throw BehaviorError("JSON_TYPE")
        }
    }
    static func checkedNumber(_ value: Double) throws -> BehaviorJSON {
        guard value.isFinite && abs(value) <= 9_007_199_254_740_991 else { throw BehaviorError("JSON_NUMBER") }; return .number(value)
    }
    func get(_ key: String) -> BehaviorJSON { if case .object(let fields) = self { return fields[key] ?? .null }; return .null }
    func has(_ key: String) -> Bool { if case .object(let fields) = self { return fields[key] != nil }; return false }
    var text: String { get throws { guard case .string(let value) = self else { throw BehaviorError("JSON_STRING") }; return value } }
    var integer: Int { get throws { guard let whole = Int(exactly: try milliseconds) else { throw BehaviorError("JSON_INTEGER") }; return whole } }
    var milliseconds: Int64 { get throws { guard case .number(let value) = self, value.isFinite, abs(value) <= 9_007_199_254_740_991, value.rounded() == value else { throw BehaviorError("JSON_INTEGER") }; return Int64(value) } }
    var number: Double { get throws { guard case .number(let value) = self, value.isFinite else { throw BehaviorError("JSON_NUMBER") }; return value } }
    var boolean: Bool { get throws { guard case .bool(let value) = self else { throw BehaviorError("JSON_BOOLEAN") }; return value } }
    var items: [BehaviorJSON] { get throws { guard case .array(let value) = self else { throw BehaviorError("JSON_ARRAY") }; return value } }
    var members: [String: BehaviorJSON] { get throws { guard case .object(let value) = self else { throw BehaviorError("JSON_OBJECT") }; return value } }
    func keys(_ allowed: [String]) throws { for key in try members.keys { guard allowed.contains(key) else { throw BehaviorError("UNKNOWN_FIELD") } } }
    func encoded(maximum: Int = 1_048_576) throws -> String { let result = try encode(0); guard result.utf8.count <= maximum else { throw BehaviorError("JSON_LIMIT") }; return result }
    private func encode(_ depth: Int) throws -> String {
        guard depth <= 32 else { throw BehaviorError("JSON_DEPTH") }
        switch self {
        case .null: return "null"
        case .bool(let value): return value ? "true" : "false"
        case .number(let value): _ = try Self.checkedNumber(value); return value.rounded() == value ? String(Int64(value)) : String(value)
        case .string(let value):
            guard value.utf8.count <= 1_048_576 else { throw BehaviorError("JSON_LIMIT") }
            return try BehaviorPlatform.quoted(value)
        case .array(let values):
            var result = "["
            for (index, value) in values.enumerated() { result += (index == 0 ? "" : ",") + (try value.encode(depth + 1)); guard result.utf8.count <= 1_048_576 else { throw BehaviorError("JSON_LIMIT") } }; return result + "]"
        case .object(let values):
            var result = "{"; var first = true
            for key in values.keys.sorted() { guard let value = values[key] else { throw BehaviorError("JSON_INVALID") }; result += (first ? "" : ",") + (try BehaviorJSON.string(key).encode(depth + 1)) + ":" + (try value.encode(depth + 1)); first = false; guard result.utf8.count <= 1_048_576 else { throw BehaviorError("JSON_LIMIT") } }; return result + "}"
        }
    }
}
private struct BehaviorJSONReader {
    let bytes: [UInt8]; var index = 0
    mutating func space() { while index < bytes.count && [9, 10, 13, 32].contains(bytes[index]) { index += 1 } }
    mutating func take(_ byte: UInt8) -> Bool { space(); if index < bytes.count && bytes[index] == byte { index += 1; return true }; return false }
    mutating func read(_ depth: Int) throws -> BehaviorJSON {
        space(); guard depth <= 32, index < bytes.count else { throw BehaviorError("JSON_INVALID") }
        if bytes[index] == 34 { return .string(try string()) }
        if take(123) {
            var fields: [String: BehaviorJSON] = [:]; if take(125) { return .object(fields) }
            repeat { let key = try string(); guard fields[key] == nil else { throw BehaviorError("JSON_DUPLICATE") }; guard take(58) else { throw BehaviorError("JSON_INVALID") }; fields[key] = try read(depth + 1) } while take(44)
            guard take(125) else { throw BehaviorError("JSON_INVALID") }; return .object(fields)
        }
        if take(91) {
            var values: [BehaviorJSON] = []; if take(93) { return .array(values) }
            repeat { values.append(try read(depth + 1)) } while take(44)
            guard take(93) else { throw BehaviorError("JSON_INVALID") }; return .array(values)
        }
        for (literal, value) in [("null", BehaviorJSON.null), ("true", .bool(true)), ("false", .bool(false))] {
            let token = Array(literal.utf8); if bytes[index...].starts(with: token) { index += token.count; return value }
        }
        let start = index
        while index < bytes.count && ![9, 10, 13, 32, 44, 93, 125].contains(bytes[index]) { index += 1 }
        guard index > start else { throw BehaviorError("JSON_INVALID") }
        do {
            return try BehaviorJSON.checkedNumber(BehaviorPlatform.number(bytes[start..<index]))
        } catch let error as BehaviorError { throw error } catch { throw BehaviorError("JSON_INVALID") }
    }
    mutating func string() throws -> String {
        space(); let start = index; guard take(34) else { throw BehaviorError("JSON_STRING") }
        var escaped = false
        while index < bytes.count {
            let byte = bytes[index]; index += 1
            if byte == 34 && !escaped {
                do { return try BehaviorPlatform.text(bytes[start..<index]) } catch { throw BehaviorError("JSON_STRING") }
            }
            if byte == 92 && !escaped { escaped = true } else { escaped = false }
        }
        throw BehaviorError("JSON_STRING")
    }
}
final class BehaviorSignal {
    private let lock = BehaviorLock(); private var cancelled = false; private let parent: BehaviorSignal?
    init() { parent = nil }
    fileprivate init(parent: BehaviorSignal) { self.parent = parent }
    var aborted: Bool { lock.lock(); let local = cancelled; lock.unlock(); return local || (parent?.aborted ?? false) }
    func abort() { lock.lock(); cancelled = true; lock.unlock() }
}
struct BehaviorExecution {
    let environment: String; let partition: String; let signal: BehaviorSignal; let owner: BehaviorSignal; let deadline: Int64
    func check() throws { guard !signal.aborted && !owner.aborted && !Task.isCancelled else { throw BehaviorError("CANCELLED") }; guard Behaviors.now < deadline else { throw BehaviorError("DEADLINE") } }
}
final class BehaviorCacheAdapter {
    let get: (String, BehaviorExecution) async throws -> BehaviorJSON
    let set: (String, BehaviorJSON, BehaviorJSON, Int64, BehaviorExecution) async throws -> Void
    let delete: (String, BehaviorExecution) async throws -> Void
    init(get: @escaping (String, BehaviorExecution) async throws -> BehaviorJSON, set: @escaping (String, BehaviorJSON, BehaviorJSON, Int64, BehaviorExecution) async throws -> Void, delete: @escaping (String, BehaviorExecution) async throws -> Void) { self.get = get; self.set = set; self.delete = delete }
}
final class BehaviorHTTPAdapter {
    let exchange: (BehaviorJSON, BehaviorExecution) async throws -> BehaviorJSON
    init(exchange: @escaping (BehaviorJSON, BehaviorExecution) async throws -> BehaviorJSON) { self.exchange = exchange }
}
final class BehaviorEntityAdapter {
    let execute: (BehaviorJSON, BehaviorExecution) async throws -> BehaviorJSON
    init(execute: @escaping (BehaviorJSON, BehaviorExecution) async throws -> BehaviorJSON) { self.execute = execute }
}
struct BehaviorAdapters {
    var caches: [String: BehaviorCacheAdapter] = [:]
    var http: [String: BehaviorHTTPAdapter] = [:]
    var entities: [String: BehaviorEntityAdapter] = [:]
    var authorize: ((BehaviorJSON, BehaviorExecution) async throws -> Bool)? = nil
    var authorizeEntity: ((BehaviorJSON, BehaviorExecution) async throws -> Bool)? = nil
}
enum BehaviorPage {
    static func normalize(_ input: BehaviorJSON, policy: BehaviorJSON) throws -> BehaviorJSON {
        let pageInput: BehaviorJSON = input == .null ? .object([:]) : input; try pageInput.keys(["page", "size", "offset", "sort"])
        let page = try pageInput.has("page") ? pageInput.get("page").integer : 0
        let size = try pageInput.has("size") ? pageInput.get("size").integer : policy.get("defaultSize").integer
        let maximum = try policy.get("maxSize").integer
        guard page >= 0, size >= 1, size <= maximum, maximum <= 1000, page <= 1_000_000 / size else { throw BehaviorError("PAGE_INVALID") }
        let allowed = try Set(policy.get("sort").items.map { try $0.text }); var seen = Set<String>(); var sort: [BehaviorJSON] = []
        for item in try pageInput.has("sort") ? pageInput.get("sort").items : [] {
            let parts = try item.text.split(separator: ":", omittingEmptySubsequences: false).map(String.init); let field = parts[0]; let direction = parts.count == 1 ? "asc" : parts[1]
            guard parts.count <= 2, allowed.contains(field), seen.insert(field).inserted, ["asc", "desc"].contains(direction) else { throw BehaviorError("PAGE_SORT") }; sort.append(.string(field + ":" + direction))
        }
        return .object(["page": .number(Double(page)), "size": .number(Double(size)), "offset": .number(Double(page * size)), "sort": .array(sort)])
    }
}
enum Behaviors {
    static var now: Int64 { BehaviorPlatform.now }
    static var module: String { __TAC_BEHAVIOR_MODULE__ }
    static var environment: String { __TAC_BEHAVIOR_ENVIRONMENT__ }
    static func answer<T: Decodable>(_ type: T.Type, _ value: BehaviorJSON) throws -> T {
        do { return try BehaviorPlatform.model(type, value) } catch { throw BehaviorError("HTTP_RESPONSE") }
    }
    static func registration(_ type: String) throws -> BehaviorJSON { let value = try BehaviorJSON.parse(__TAC_BEHAVIOR_REGISTRY__).get(type); guard value != .null else { throw BehaviorError("UNREGISTERED") }; return value }
    static func entity(_ entity: Any.Type) throws -> BehaviorJSON {
        __TAC_BEHAVIOR_ENTITIES__
        throw BehaviorError("ENTITY_UNKNOWN")
    }
    private static func typePolicies(_ type: Any.Type) throws -> BehaviorJSON {
        __TAC_BEHAVIOR_TYPES__
        return .object([:])
    }
    static func validate(_ type: Any.Type, _ value: BehaviorJSON) throws -> BehaviorJSON {
        do { return try validateSchema(typePolicies(type).get("validated"), value) }
        catch let error as BehaviorError where error.code == "validation" { throw error }
        catch { throw BehaviorError("validation", path: "$.schema") }
    }
    static func validateSchema(_ schema: BehaviorJSON, _ value: BehaviorJSON) throws -> BehaviorJSON {
        do { _ = try value.encoded(maximum: 1048576) } catch { throw BehaviorError("validation", path: "$.json") }
        guard case .object = value else { throw BehaviorError("validation", path: "$.object") }
        do {
            try schema.keys(["fields"])
            for (field, rules) in try schema.get("fields").members {
                guard matches(field, "[A-Za-z_$][A-Za-z0-9_$]{0,63}") else { throw BehaviorError("validation", path: "$.schema") }
                try rules.keys(["notNull", "notBlank", "size", "min", "max"]); let item = value.get(field)
                if rules.has("notNull"), try rules.get("notNull").boolean, item == .null { throw BehaviorError("validation", path: field + ".notNull") }
                if rules.has("notBlank"), try rules.get("notBlank").boolean {
                    guard case .string(let text) = item, !isBlank(text) else { throw BehaviorError("validation", path: field + ".notBlank") }
                }
                if item == .null { continue }
                if rules.has("size") {
                    let size = rules.get("size"); try size.keys(["min", "max"])
                    let count: Int
                    switch item { case .string(let value): count = value.utf8.count; case .array(let value): count = value.count; case .object(let value): count = value.count; default: count = -1 }
                    let minimum = try size.has("min") ? size.get("min").integer : 0, maximum = try size.has("max") ? size.get("max").integer : 1048576
                    guard minimum >= 0, maximum >= minimum else { throw BehaviorError("validation", path: "$.schema") }
                    guard count >= minimum, count <= maximum else { throw BehaviorError("validation", path: field + ".size") }
                }
                for rule in ["min", "max"] where rules.has(rule) {
                    guard case .number(let number) = item else { throw BehaviorError("validation", path: field + "." + rule) }
                    guard case .number(let bound) = rules.get(rule) else { throw BehaviorError("validation", path: "$.schema") }; _ = try BehaviorJSON.checkedNumber(bound)
                    guard !(rule == "min" && number < bound), !(rule == "max" && number > bound) else { throw BehaviorError("validation", path: field + "." + rule) }
                }
            }
            return value
        } catch let error as BehaviorError where error.code == "validation" { throw error }
        catch { throw BehaviorError("validation", path: "$.schema") }
    }
    private static func isBlank(_ value: String) -> Bool {
        value.unicodeScalars.allSatisfy { scalar in let code = scalar.value; return (9...13).contains(code) || [32, 133, 160, 5760, 8232, 8233, 8239, 8287, 12288].contains(code) || (8192...8202).contains(code) }
    }
    private static func configure(_ configuration: Any.Type, execution: BehaviorExecution) throws -> BehaviorAdapters {
        __TAC_BEHAVIOR_CONFIGURATIONS__
        throw BehaviorError("CONFIGURATION_UNKNOWN")
    }
    static func create(_ configuration: Any.Type, partition: String, signal: BehaviorSignal, timeoutMs: Int = 10000, profiles: [String] = [], properties: [String: String] = [:]) throws -> BehaviorContext {
        guard !partition.isEmpty, partition.utf8.count <= 256, (1...60000).contains(timeoutMs) else { throw BehaviorError("CONTEXT_INVALID") }
        let execution = BehaviorExecution(environment: environment, partition: partition, signal: signal, owner: signal, deadline: now + Int64(timeoutMs)); try execution.check()
        let selected = try snapshotProfiles(profiles); let values = try snapshotProperties(properties, code: "configuration")
        try activeConfiguration(typePolicies(configuration), profiles: selected, values: values)
        let setupKey = String(reflecting: configuration) + "\(selected.sorted())\(values.sorted { $0.key < $1.key }.map { "\($0.key)=\($0.value)" })"
        let adapters = try acquireSetup(setupKey, configuration, execution: execution)
        return BehaviorContext(adapters: adapters, partition: partition, owner: signal, timeoutMs: timeoutMs, setupKey: setupKey)
    }
    private final class SharedSetup {
        let adapters: BehaviorAdapters; var users = 1
        init(_ adapters: BehaviorAdapters) { self.adapters = adapters }
    }
    nonisolated(unsafe) private static var setups: [String: SharedSetup] = [:]
    nonisolated(unsafe) private static let setupLock = BehaviorLock()
    private static func acquireSetup(_ setupKey: String, _ configuration: Any.Type, execution: BehaviorExecution) throws -> BehaviorAdapters {
        setupLock.lock(); defer { setupLock.unlock() }
        if let shared = setups[setupKey] { shared.users += 1; return shared.adapters }
        let adapters: BehaviorAdapters
        do { adapters = try configure(configuration, execution: execution) } catch let error as BehaviorError { throw error } catch { throw BehaviorError("CONFIGURATION_FAILED") }
        try execution.check()
        setups[setupKey] = SharedSetup(adapters)
        return adapters
    }
    static func releaseSetup(_ setupKey: String) {
        setupLock.lock(); defer { setupLock.unlock() }
        guard let shared = setups[setupKey] else { return }
        shared.users -= 1
        if shared.users == 0 { setups[setupKey] = nil }
    }
    private static var frameworkPrefix: String { environment == "tac" ? "TAC_" : "YON_" }
    private static func matches(_ value: String, _ pattern: String) -> Bool { BehaviorPlatform.matches(value, pattern) }
    private static func snapshotProfiles(_ profiles: [String]) throws -> [String] {
        guard profiles.count <= 32 else { throw BehaviorError("configuration") }; var seen = Set<String>()
        for profile in profiles { guard matches(profile, "[A-Za-z_$][A-Za-z0-9_$]{0,63}"), seen.insert(profile).inserted else { throw BehaviorError("configuration") } }; return Array(profiles)
    }
    private static func snapshotProperties(_ values: [String: String], code: String) throws -> [String: String] {
        do {
            guard values.count <= 256 else { throw BehaviorError(code) }; var copy: [String: String] = [:]; var json: [String: BehaviorJSON] = [:]
            for (key, value) in values { guard key.utf8.count <= 256, key.hasPrefix(frameworkPrefix), matches(key, "[A-Z][A-Z0-9_]*"), value.utf8.count <= 4096 else { throw BehaviorError(code) }; copy[key] = value; json[key] = .string(value) }
            _ = try BehaviorJSON.object(json).encoded(maximum: 65536); return copy
        } catch { throw BehaviorError(code) }
    }
    private static func activeConfiguration(_ policies: BehaviorJSON, profiles: [String], values: [String: String]) throws {
        if policies.has("profiles") {
            let allowed = try policies.get("profiles").items; guard !allowed.isEmpty, allowed.count <= 32 else { throw BehaviorError("configuration") }; var active = false
            for profile in allowed { let name = try profile.text; guard matches(name, "[A-Za-z_$][A-Za-z0-9_$]{0,63}") else { throw BehaviorError("configuration") }; if profiles.contains(name) { active = true } }
            guard active else { throw BehaviorError("inactive-configuration") }
        }
        if policies.has("conditionalOnProperty") {
            let condition = policies.get("conditionalOnProperty"); try condition.keys(["name", "havingValue", "matchIfMissing"])
            let name = try condition.get("name").text; let expected = try condition.get("havingValue").text
            guard name.utf8.count <= 256, name.hasPrefix(frameworkPrefix), matches(name, "[A-Z][A-Z0-9_]*"), expected.utf8.count <= 4096 else { throw BehaviorError("configuration") }
            if values.keys.contains(where: { $0.hasPrefix(name + "_") }) { throw BehaviorError("inactive-configuration") }
            let active = try values[name].map { $0 == expected } ?? condition.get("matchIfMissing").boolean
            guard active else { throw BehaviorError("inactive-configuration") }
        }
    }
    static func properties(_ type: Any.Type, _ values: [String: String]) throws -> BehaviorJSON {
        do {
            let supplied = try snapshotProperties(values, code: "configuration-properties"); let schema = try typePolicies(type).get("configurationProperties"); try schema.keys(["prefix", "fields"])
            let prefix = try schema.get("prefix").text
            guard prefix.utf8.count <= 128, prefix.hasPrefix(frameworkPrefix), prefix.hasSuffix("_"), matches(prefix, "[A-Z][A-Z0-9_]*") else { throw BehaviorError("configuration-properties") }
            let fields = try schema.get("fields").members; var known = Set<String>(); var result: [String: BehaviorJSON] = [:]
            for (name, descriptor) in fields {
                guard matches(name, "[A-Za-z_$][A-Za-z0-9_$]{0,63}") else { throw BehaviorError("configuration-properties") }; try descriptor.keys(["name", "type", "default", "min", "max"])
                let suffix = try descriptor.get("name").text; let kind = try descriptor.get("type").text
                guard matches(suffix, "[A-Z][A-Z0-9_]*"), (prefix + suffix).utf8.count <= 256, known.insert(prefix + suffix).inserted else { throw BehaviorError("configuration-properties") }
                let value: BehaviorJSON
                if let raw = supplied[prefix + suffix] { value = try parseProperty(raw, type: kind) }
                else { guard descriptor.has("default") else { throw BehaviorError("configuration-properties") }; value = descriptor.get("default") }
                try validateProperty(value, descriptor: descriptor); result[name] = value
            }
            for key in supplied.keys { if key.hasPrefix(prefix) && !known.contains(key) { throw BehaviorError("configuration-properties") } }
            let output = BehaviorJSON.object(result); _ = try output.encoded(maximum: 65536); return output
        } catch { throw BehaviorError("configuration-properties") }
    }
    private static func applicationInputs(_ application: BehaviorJSON) throws -> [String: String] {
        var flat: [String: String] = [:]; _ = try application.encoded(maximum: 65536)
        let members = try application.members
        guard members["tac"] != nil || members["yon"] != nil else { throw BehaviorError("configuration-properties") }
        for (key, value) in members {
            guard key == "tac" || key == "yon" else { continue }
            _ = try value.members; try flattenApplication(value, path: key, depth: 0, flat: &flat)
        }
        return flat
    }
    private static func flattenApplication(_ value: BehaviorJSON, path: String, depth: Int, flat: inout [String: String]) throws {
        guard depth <= 16, path.utf8.count <= 256, flat.count < 256 else { throw BehaviorError("configuration-properties") }
        if path == "yon.server.port" { _ = try value.integer }
        if case .object(let values) = value {
            if depth > 0 && values.isEmpty { flat[path + "."] = "" }
            for (key, child) in values {
                guard matches(key, "[a-z][a-z0-9]*") else { throw BehaviorError("configuration-properties") }
                try flattenApplication(child, path: path + "." + key, depth: depth + 1, flat: &flat)
            }
        } else {
            let text: String
            switch value {
            case .null, .array: flat[path + "."] = ""; return
            case .string(let raw): text = raw
            case .bool(let raw): text = raw ? "true" : "false"
            case .number(let raw): text = raw.rounded() == raw && abs(raw) <= 9_007_199_254_740_991 ? String(Int64(raw)) : String(raw)
            default: throw BehaviorError("configuration-properties")
            }
            guard text.utf8.count <= 4096, flat.count < 256 else { throw BehaviorError("configuration-properties") }
            flat[path] = text
        }
    }
    private static func applicationProperties(_ application: BehaviorJSON) throws -> [String: String] {
        var result: [String: String] = [:]
        for (path, value) in try applicationInputs(application) where path.hasPrefix(environment + ".") {
            result[BehaviorPlatform.replaced(path, ".", "_").uppercased()] = value
        }
        return result
    }
    static func properties(_ type: Any.Type, _ application: BehaviorJSON) throws -> BehaviorJSON {
        try properties(type, applicationProperties(application))
    }
    static func create(_ configuration: Any.Type, partition: String, signal: BehaviorSignal, timeoutMs: Int = 10000, profiles: [String] = [], properties: BehaviorJSON) throws -> BehaviorContext {
        try create(configuration, partition: partition, signal: signal, timeoutMs: timeoutMs, profiles: profiles, properties: applicationProperties(properties))
    }
    static func values(_ type: Any.Type, _ application: BehaviorJSON) throws -> BehaviorJSON {
        do {
            let flat = try applicationInputs(application); let schema = try typePolicies(type).get("values").members; var result: [String: BehaviorJSON] = [:]
            for (name, descriptor) in schema {
                guard matches(name, "[A-Za-z_$][A-Za-z0-9_$]{0,127}") else { throw BehaviorError("configuration-properties") }; try descriptor.keys(["path", "type", "default", "min", "max"])
                let path = try descriptor.get("path").text; let kind = try descriptor.get("type").text
                let value: BehaviorJSON
                if flat.keys.contains(where: { $0.hasPrefix(path + ".") }) { throw BehaviorError("configuration-properties") }
                if let raw = flat[path] { value = try parseProperty(raw, type: kind) }
                else { guard descriptor.has("default") else { throw BehaviorError("configuration-properties") }; value = descriptor.get("default") }
                try validateProperty(value, descriptor: descriptor); result[name] = value
            }
            let output = BehaviorJSON.object(result); _ = try output.encoded(maximum: 65536); return output
        } catch { throw BehaviorError("configuration-properties") }
    }
    private static func parseProperty(_ raw: String, type: String) throws -> BehaviorJSON {
        if type == "string" { return .string(raw) }
        if type == "boolean" { guard raw == "true" || raw == "false" else { throw BehaviorError("configuration-properties") }; return .bool(raw == "true") }
        let grammar = type == "integer" ? "-?(0|[1-9][0-9]*)" : "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?"
        guard ["integer", "number"].contains(type), matches(raw, grammar) else { throw BehaviorError("configuration-properties") }; return try BehaviorJSON.parse(raw)
    }
    private static func validateProperty(_ value: BehaviorJSON, descriptor: BehaviorJSON) throws {
        let type = try descriptor.get("type").text; let numeric = type == "integer" || type == "number"
        guard numeric || (!descriptor.has("min") && !descriptor.has("max")) else { throw BehaviorError("configuration-properties") }
        if type == "string" { guard try value.text.utf8.count <= 4096 else { throw BehaviorError("configuration-properties") }; return }
        if type == "boolean" { _ = try value.boolean; return }
        guard numeric, case .number(let number) = value else { throw BehaviorError("configuration-properties") }; _ = try BehaviorJSON.checkedNumber(number)
        if type == "integer" { _ = try value.integer }
        var minimum = -9_007_199_254_740_991.0; var maximum = 9_007_199_254_740_991.0
        if descriptor.has("min") { guard case .number(let value) = descriptor.get("min") else { throw BehaviorError("configuration-properties") }; _ = try BehaviorJSON.checkedNumber(value); minimum = value }
        if descriptor.has("max") { guard case .number(let value) = descriptor.get("max") else { throw BehaviorError("configuration-properties") }; _ = try BehaviorJSON.checkedNumber(value); maximum = value }
        guard minimum <= maximum, number >= minimum, number <= maximum else { throw BehaviorError("configuration-properties") }
    }
}
struct BehaviorBrowserRead: Sendable {
    let publicRead: Bool; let revalidate: Bool
    nonisolated static let currentBinding = TaskLocal(wrappedValue: BehaviorBrowserRead(publicRead: false, revalidate: false))
    static var current: BehaviorBrowserRead { currentBinding.get() }
}
enum BehaviorOpenTransactions {
    nonisolated static let openBinding = TaskLocal<[String: String]>(wrappedValue: [:])
    static var open: [String: String] { openBinding.get() }
    static func within(_ adapter: String, _ identifier: String, _ body: () async throws -> BehaviorJSON) async throws -> BehaviorJSON {
        var joined = open
        joined[adapter] = identifier
        return try await openBinding.withValue(joined) { try await body() }
    }
}
final class BehaviorContext {
    private let adapters: BehaviorAdapters; private let partition: String; private let owner: BehaviorSignal; private let signal: BehaviorSignal; private let timeoutMs: Int
    private let lock = BehaviorLock(); private var active = 0; private var slots: [String: Int] = [:]
    private let setupKey: String; private var released = false; private var revalidations: Set<String> = []
    init(adapters: BehaviorAdapters, partition: String, owner: BehaviorSignal, timeoutMs: Int, setupKey: String) { self.adapters = adapters; self.partition = partition; self.owner = owner; self.signal = BehaviorSignal(parent: owner); self.timeoutMs = timeoutMs; self.setupKey = setupKey }
    func dispose() {
        signal.abort()
        lock.lock(); let first = !released; released = true; lock.unlock()
        if first { Behaviors.releaseSetup(setupKey) }
    }
    private func markRevalidation(_ key: String) { lock.lock(); revalidations.insert(key); lock.unlock() }
    private func takeRevalidation(_ key: String) -> Bool { lock.lock(); defer { lock.unlock() }; return revalidations.remove(key) != nil }
    private func acquire() throws { lock.lock(); defer { lock.unlock() }; guard active < 64 else { throw BehaviorError("OPERATION_LIMIT") }; active += 1 }
    private func release() { lock.lock(); active -= 1; lock.unlock() }
    func paginate<T>(_ body: () throws -> T) throws -> T {
        try acquire(); defer { release() }; let execution = BehaviorExecution(environment: Behaviors.environment, partition: partition, signal: signal, owner: owner, deadline: Behaviors.now + Int64(timeoutMs))
        try execution.check(); let result = try body(); try execution.check(); return result
    }
    private func operation(_ body: (BehaviorExecution) async throws -> BehaviorJSON) async throws -> BehaviorJSON {
        try acquire(); defer { release() }; let execution = BehaviorExecution(environment: Behaviors.environment, partition: partition, signal: signal, owner: owner, deadline: Behaviors.now + Int64(timeoutMs))
        try execution.check(); let result = try await body(execution); try execution.check(); _ = try result.encoded(); return result
    }
    private func adapter<T>(_ execution: BehaviorExecution, _ body: () async throws -> T) async throws -> T {
        try execution.check(); let result: T
        do { result = try await body() } catch let error as BehaviorError { throw error } catch { try execution.check(); throw BehaviorError("ADAPTER_FAILED") }; try execution.check(); return result
    }
    private func cache(_ name: String) throws -> BehaviorCacheAdapter {
        guard let cache = adapters.caches[name] ?? BehaviorPlatform.builtinCache(name) else { throw BehaviorError("CACHE_MISSING") }
        return cache
    }
    private func entities(_ name: String) throws -> BehaviorEntityAdapter {
        guard let adapter = adapters.entities[name] ?? BehaviorPlatform.builtinEntities(name) else { throw BehaviorError("ENTITY_MISSING") }
        return adapter
    }
    private func builtinCache(_ name: String) -> Bool { adapters.caches[name] == nil && BehaviorPlatform.builtinCache(name) != nil }
    private func builtinEntities(_ name: String) -> Bool { adapters.entities[name] == nil && BehaviorPlatform.builtinEntities(name) != nil }
    private func authorizeEntity(_ name: String, _ command: BehaviorJSON, _ execution: BehaviorExecution) async throws {
        guard let authorize = adapters.authorizeEntity else { guard builtinEntities(name) else { throw BehaviorError("FORBIDDEN") }; return }
        guard try await adapter(execution, { try await authorize(command, execution) }) else { throw BehaviorError("FORBIDDEN") }
    }
    private func method(_ type: String, _ method: String) throws -> BehaviorJSON { let metadata = try Behaviors.registration(type).get("methods").get(method); guard metadata != .null else { throw BehaviorError("METHOD_UNKNOWN") }; return metadata }
    private func authorize(_ operation: String, _ type: String, _ method: String, policy: BehaviorJSON, metadata: BehaviorJSON, arguments: [BehaviorJSON], execution: BehaviorExecution) async throws -> String {
        let selected = try metadata.get("cacheable").get("keyArgs").items.map { item -> BehaviorJSON in let index = try item.integer; guard arguments.indices.contains(index) else { throw BehaviorError("CACHE_KEY") }; return arguments[index] }
        let key = try BehaviorJSON.array([.string(partition), policy.get("name"), .string(Behaviors.module), .string(type), .string(method), .array(selected)]).encoded(maximum: 4096)
        let call = BehaviorJSON.object(["operation": .string(operation), "cache": policy.get("name"), "type": .string(type), "method": .string(method), "keyArguments": .array(selected)])
        guard let authorize = adapters.authorize else { guard try builtinCache(policy.get("name").text) else { throw BehaviorError("FORBIDDEN") }; return key }; guard try await adapter(execution, { try await authorize(call, execution) }) else { throw BehaviorError("FORBIDDEN") }; return key
    }
    func invoke(_ type: String, _ method: String, arguments: [BehaviorJSON], body: () async throws -> BehaviorJSON) async throws -> BehaviorJSON {
        try await operation { execution in
            let written = try self.method(type, method)
            guard written.get("httpExchange").has("generated") else { return try await self.planned(type, method, written, arguments: arguments, body: body, execution: execution) }
            let declared = try Self.exchangeRequest(written.get("httpExchange"), arguments)
            let metadata = try Self.withFilledPath(written, arguments)
            guard ["cacheable", "cacheEvict", "cachePut"].contains(where: { metadata.has($0) }) else { return try await self.body(metadata, body: { declared }, execution: execution) }
            return try await self.planned(type, method, metadata, arguments: arguments, body: { declared }, execution: execution)
        }
    }
    private func planned(_ type: String, _ method: String, _ metadata: BehaviorJSON, arguments: [BehaviorJSON], body: () async throws -> BehaviorJSON, execution: BehaviorExecution) async throws -> BehaviorJSON {
        if metadata.has("cacheEvict") || metadata.has("cachePut") { return try await self.mutation(type, metadata: metadata, arguments: arguments, body: body, execution: execution) }
        guard metadata.has("cacheable") else { return try await self.resilient(type, method, metadata, body: body, execution: execution) }
        let policy = try Behaviors.registration(type).get("cache"); let key = try await self.authorize("cache:read", type, method, policy: policy, metadata: metadata, arguments: arguments, execution: execution)
        let cache = try self.cache(policy.get("name").text)
        let entry = try await self.adapter(execution, { try await cache.get(key, execution) }); _ = try entry.encoded(); try entry.keys(["hit", "value", "expiresAt"])
        if try entry.get("hit").boolean {
            let expiry = try entry.get("expiresAt").milliseconds; guard expiry <= Behaviors.now + (try policy.get("ttlMs").milliseconds) else { throw BehaviorError("CACHE_EXPIRY") }
            if expiry > Behaviors.now { guard entry.has("value") else { throw BehaviorError("CACHE_VALUE") }; let value = entry.get("value"); _ = try value.encoded(maximum: policy.get("maxBytes").integer); return value }
        }
        let reading = BehaviorBrowserRead(publicRead: metadata.get("httpExchange").get("method") == .string("GET"), revalidate: takeRevalidation(key))
        let result = try await BehaviorBrowserRead.currentBinding.withValue(reading) { try await self.resilient(type, method, metadata, body: body, execution: execution) }; try execution.check(); _ = try result.encoded(maximum: policy.get("maxBytes").integer)
        try await self.adapter(execution, { try await cache.set(key, result, policy, Behaviors.now + policy.get("ttlMs").milliseconds, execution) }); return result
    }
    private static func withFilledPath(_ metadata: BehaviorJSON, _ arguments: [BehaviorJSON]) throws -> BehaviorJSON {
        let policy = metadata.get("httpExchange")
        guard policy.has("pathVariables") else { return metadata }
        var path = try policy.get("path").text
        for (name, index) in try policy.get("pathVariables").members {
            let at = try index.integer
            guard arguments.indices.contains(at) else { throw BehaviorError("HTTP_ARGUMENT") }
            let text: String
            switch arguments[at] {
            case .string(let value): text = value
            case .bool(let value): text = value ? "true" : "false"
            case .number(let value): text = value.rounded() == value && abs(value) < 1e15 ? String(Int64(value)) : String(value)
            default: throw BehaviorError("HTTP_ARGUMENT")
            }
            let allowed = !text.isEmpty && text.utf8.count <= 256 && text != "." && text != ".." && text.utf8.allSatisfy { byte in
                (byte >= 48 && byte <= 57) || (byte >= 65 && byte <= 90) || (byte >= 97 && byte <= 122) || byte == 46 || byte == 95 || byte == 126 || byte == 45
            }
            guard allowed else { throw BehaviorError("HTTP_ARGUMENT") }
            path = BehaviorPlatform.replaced(path, "{\(name)}", text)
        }
        var exchange = try policy.members; exchange["path"] = .string(path)
        var copy = try metadata.members; copy["httpExchange"] = .object(exchange)
        return .object(copy)
    }
    private static func exchangeRequest(_ policy: BehaviorJSON, _ arguments: [BehaviorJSON]) throws -> BehaviorJSON {
        func pick(_ index: Int) throws -> BehaviorJSON {
            guard arguments.indices.contains(index) else { throw BehaviorError("HTTP_ARGUMENT") }
            return arguments[index]
        }
        var query: [String: BehaviorJSON] = [:]
        var headers: [String: BehaviorJSON] = [:]
        if policy.has("query") { for (name, index) in try policy.get("query").members { query[name] = try pick(try index.integer) } }
        if policy.has("headers") { for (name, index) in try policy.get("headers").members { headers[name] = try pick(try index.integer) } }
        var descriptor: [String: BehaviorJSON] = ["query": .object(query), "headers": .object(headers)]
        if policy.has("body") { descriptor["body"] = try pick(try policy.get("body").integer) }
        return .object(descriptor)
    }
    private func mutationArguments(_ mutation: BehaviorJSON, arguments: [BehaviorJSON]) throws -> [BehaviorJSON] {
        var normalized = try mutation.get("arguments").items.map { item -> BehaviorJSON in let index = try item.integer; guard arguments.indices.contains(index) else { throw BehaviorError("CACHE_ARGUMENT") }; return arguments[index] }
        let target = mutation.get("target")
        if target.has("pageable") { let page = target.get("pageable"); let index = try page.get("argument").integer; guard normalized.indices.contains(index) else { throw BehaviorError("PAGE_ARGUMENT") }; normalized[index] = try BehaviorPage.normalize(normalized[index], policy: page) }
        return normalized
    }
    private func mutation(_ type: String, metadata: BehaviorJSON, arguments: [BehaviorJSON], body: () async throws -> BehaviorJSON, execution: BehaviorExecution) async throws -> BehaviorJSON {
        let put = metadata.has("cachePut"); let mutation = metadata.get(put ? "cachePut" : "cacheEvict"); let target = mutation.get("target")
        let policy = try Behaviors.registration(type).get("cache"); let normalized = try mutationArguments(mutation, arguments: arguments)
        let key = try await authorize(put ? "cache:put" : "cache:evict", type, target.get("method").text, policy: policy, metadata: target, arguments: normalized, execution: execution)
        let cache = try self.cache(policy.get("name").text)
        let result = try await self.resilient(type, try target.get("method").text, metadata, body: body, execution: execution); try execution.check(); _ = try result.encoded(maximum: put ? policy.get("maxBytes").integer : 1_048_576)
        if put { try await adapter(execution, { try await cache.set(key, result, policy, Behaviors.now + policy.get("ttlMs").milliseconds, execution) }) }
        else { try await adapter(execution, { try await cache.delete(key, execution) }) }
        markRevalidation(key)
        return result
    }
    private func enter(_ key: String, limit: Int) -> Bool {
        lock.lock(); defer { lock.unlock() }
        let taken = slots[key] ?? 0
        guard taken < limit else { return false }
        slots[key] = taken + 1
        return true
    }
    private func leave(_ key: String) {
        lock.lock(); defer { lock.unlock() }
        let taken = (slots[key] ?? 1) - 1
        if taken <= 0 { slots.removeValue(forKey: key) } else { slots[key] = taken }
    }
    private func retryable(_ error: Error) -> Bool { (error as? BehaviorError).map { $0.code == "ADAPTER_FAILED" } ?? true }
    private func transacted(_ metadata: BehaviorJSON, body: () async throws -> BehaviorJSON, execution: BehaviorExecution) async throws -> BehaviorJSON {
        guard metadata.has("transactional") else { return try await attempt(metadata, body: body, execution: execution) }
        let name = try metadata.get("transactional").get("adapter").text
        guard BehaviorOpenTransactions.open[name] == nil else { return try await attempt(metadata, body: body, execution: execution) }
        let adapter = try entities(name)
        let control = { (action: String, identifier: String?) async throws -> BehaviorJSON in
            var command: [String: BehaviorJSON] = ["operation": .string("transaction"), "action": .string(action)]
            if let identifier { command["transaction"] = .string(identifier) }
            return try await self.adapter(execution, { try await adapter.execute(.object(command), execution) })
        }
        let begun = try await control("begin", nil)
        let identifier = begun.has("transaction") ? (try? begun.get("transaction").text) ?? "" : ""
        guard !identifier.isEmpty, identifier.utf8.count <= 128 else { throw BehaviorError("ENTITY_TRANSACTION") }
        let value: BehaviorJSON
        do { value = try await BehaviorOpenTransactions.within(name, identifier) { try await attempt(metadata, body: body, execution: execution) } }
        catch {
            _ = try? await control("rollback", identifier)
            throw error
        }
        _ = try await control("commit", identifier)
        return value
    }
    private func resilient(_ type: String, _ method: String, _ metadata: BehaviorJSON, body: () async throws -> BehaviorJSON, execution: BehaviorExecution) async throws -> BehaviorJSON {
        guard metadata.has("concurrencyLimit") else { return try await transacted(metadata, body: body, execution: execution) }
        let policy = metadata.get("concurrencyLimit"); let key = type + "." + method
        let limit = try policy.get("limit").integer; let blocking = try policy.get("policy").text == "block"
        while !enter(key, limit: limit) {
            guard blocking else { throw BehaviorError("OPERATION_LIMIT") }
            try execution.check()
            try? await Task.sleep(nanoseconds: 2_000_000)
            try execution.check()
        }
        defer { leave(key) }
        return try await transacted(metadata, body: body, execution: execution)
    }
    private func attempt(_ metadata: BehaviorJSON, body: () async throws -> BehaviorJSON, execution: BehaviorExecution) async throws -> BehaviorJSON {
        guard metadata.has("retryable") else { return try await self.body(metadata, body: body, execution: execution) }
        let policy = metadata.get("retryable"); let maxRetries = try policy.get("maxRetries").integer
        var attempt = 0
        while true {
            do { return try await self.body(metadata, body: body, execution: execution) }
            catch {
                guard attempt < maxRetries, retryable(error) else { throw error }
                try execution.check()
                let delay = min(Double(try policy.get("delayMs").integer) * BehaviorPlatform.power(try policy.get("multiplier").number, Double(attempt)), Double(try policy.get("maxDelayMs").integer))
                let wait = delay + Double.random(in: 0...1) * Double(try policy.get("jitterMs").integer)
                if wait > 0 { try? await Task.sleep(nanoseconds: UInt64(wait.rounded() * 1_000_000)) }
                try execution.check()
                attempt += 1
            }
        }
    }
    private func body(_ metadata: BehaviorJSON, body: () async throws -> BehaviorJSON, execution: BehaviorExecution) async throws -> BehaviorJSON {
        try execution.check(); let result = try await body(); try execution.check(); _ = try result.encoded()
        guard metadata.has("httpExchange") else { return result }; try result.keys(["query", "headers", "body"]); try validateHTTP(result)
        let policy = metadata.get("httpExchange"); if ["GET", "HEAD"].contains(try policy.get("method").text) && result.has("body") { throw BehaviorError("HTTP_BODY") }
        var request = try result.members; if request["query"] == nil { request["query"] = .object([:]) }; if request["headers"] == nil { request["headers"] = .object([:]) }; request["method"] = policy.get("method"); request["path"] = policy.get("path"); request["redirect"] = .string("error"); request["credentials"] = .string("omit"); request["deadline"] = .number(Double(execution.deadline)); request["maxResponseBytes"] = .number(1_048_576)
        guard let client = adapters.http[try policy.get("client").text] else {
            let reading = BehaviorBrowserRead.current; request["publicRead"] = .bool(reading.publicRead); request["revalidate"] = .bool(reading.revalidate)
            return try await adapter(execution, { [request] in try await BehaviorPlatform.exchange(.object(request)) })
        }
        return try await adapter(execution, { try await client.exchange(.object(request), execution) })
    }
    private func validateHTTP(_ value: BehaviorJSON) throws {
        if value.has("headers") { for (key, value) in try value.get("headers").members {
            let text = try value.text; let allowed = Set("!#$%&'*+.^_`|~0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-")
            guard !key.isEmpty, key.utf8.count <= 128, key.allSatisfy({ allowed.contains($0) }), text.utf8.count <= 8192, !text.unicodeScalars.contains(where: { $0.value == 13 || $0.value == 10 || $0.value == 0 }), !["host", "connection", "content-length", "transfer-encoding", "cookie"].contains(key.lowercased()) else { throw BehaviorError("HTTP_HEADER") }
        } }
        if value.has("query") { for (key, value) in try value.get("query").members { guard key.utf8.count <= 256 else { throw BehaviorError("HTTP_QUERY") }; _ = try value.encoded(maximum: 8192); switch value { case .object, .array, .null: throw BehaviorError("HTTP_QUERY"); default: break } } }
    }
    func evict(_ type: String, _ method: String, arguments: [BehaviorJSON]) async throws -> BehaviorJSON {
        try await operation { execution in
            let metadata = try self.method(type, method); guard metadata.has("cacheable") else { throw BehaviorError("CACHE_METHOD") }; var arguments = arguments
            if metadata.has("pageable") { let page = metadata.get("pageable"); let index = try page.get("argument").integer; guard arguments.indices.contains(index) else { throw BehaviorError("PAGE_ARGUMENT") }; arguments[index] = try BehaviorPage.normalize(arguments[index], policy: page) }
            let policy = try Behaviors.registration(type).get("cache"); let key = try await self.authorize("cache:evict", type, method, policy: policy, metadata: metadata, arguments: arguments, execution: execution)
            let cache = try self.cache(policy.get("name").text); try await self.adapter(execution, { try await cache.delete(key, execution) }); return .null
        }
    }
    func restClient(_ name: String) throws -> BehaviorRestClient {
        guard !name.isEmpty, name.utf8.count <= 128 else { throw BehaviorError("HTTP_MISSING") }
        return BehaviorRestClient(context: self, name: name)
    }
    fileprivate func restExchange(_ name: String, _ method: String, _ path: String, _ descriptor: BehaviorJSON) async throws -> BehaviorJSON {
        try await self.operation { execution in
            try descriptor.keys(["query", "headers", "body"])
            try self.validateHTTP(descriptor)
            if ["GET", "HEAD"].contains(method), descriptor.has("body") { throw BehaviorError("HTTP_BODY") }
            guard let client = self.adapters.http[name] else { throw BehaviorError("HTTP_MISSING") }
            var request = try descriptor.members
            if request["query"] == nil { request["query"] = .object([:]) }
            if request["headers"] == nil { request["headers"] = .object([:]) }
            request["method"] = .string(method); request["path"] = .string(path)
            request["redirect"] = .string("error"); request["credentials"] = .string("omit")
            request["deadline"] = .number(Double(execution.deadline)); request["maxResponseBytes"] = .number(1_048_576)
            return try await self.adapter(execution, { try await client.exchange(.object(request), execution) })
        }
    }
    fileprivate static func safePath(_ path: String) -> Bool {
        guard path.hasPrefix("/"), !path.hasPrefix("//"), path.utf8.count <= 2048 else { return false }
        if path.unicodeScalars.contains(where: { $0.value <= 32 || "\\?#%".unicodeScalars.contains($0) }) { return false }
        return !path.split(separator: "/", omittingEmptySubsequences: false).contains { $0 == "." || $0 == ".." }
    }
    func repository(_ entity: Any.Type) throws -> BehaviorRepository { BehaviorRepository(context: self, mapping: try Behaviors.entity(entity)) }
    static func mapValue(_ plan: BehaviorJSON, _ value: BehaviorJSON) throws -> BehaviorJSON {
        guard case .object = value else { throw BehaviorError("JSON_OBJECT") }
        var output: [String: BehaviorJSON] = [:]
        for field in try plan.get("fields").items {
            let target = try field.get("target").text
            guard case .string(let path) = field.get("source") else {
                output[target] = field.get("constant")
                continue
            }
            var current = value
            for name in path.split(separator: ".", omittingEmptySubsequences: false) {
                current = current.get(String(name))
            }
            output[target] = field.has("fields") ? try mapShape(current, field.get("fields")) : current
        }
        let result = BehaviorJSON.object(output); _ = try result.encoded(); return result
    }
    private static func mapShape(_ value: BehaviorJSON, _ fields: BehaviorJSON) throws -> BehaviorJSON {
        if case .array(let items) = value { return .array(try items.map { try mapShape($0, fields) }) }
        if case .object = value { return try mapValue(.object(["fields": fields]), value) }
        return value
    }
    func query(_ metadata: BehaviorJSON, arguments: [BehaviorJSON]) async throws -> BehaviorJSON {
        try await operation { execution in
            let command = try self.queryCommand(metadata, arguments: arguments)
            let name = try metadata.get("entity").get("adapter").text
            try await self.authorizeEntity(name, command, execution)
            let adapter = try self.entities(name)
            let result = try await self.adapter(execution, { try await adapter.execute(command, execution) }); _ = try result.encoded()
            switch try metadata.get("operation").text {
            case "find": let rows = try result.items; guard rows.count <= (try command.get("query").get("page").get("size").integer) else { throw BehaviorError("QUERY_RESULT") }; for row in rows { _ = try row.members }
            case "findOne": let rows = try result.items; guard rows.count <= 1 else { throw BehaviorError("QUERY_RESULT") }; for row in rows { _ = try row.members }; return rows.first ?? .null
            case "count", "delete": guard try result.integer >= 0 else { throw BehaviorError("QUERY_RESULT") }
            case "exists": _ = try result.boolean
            default: throw BehaviorError("QUERY_KIND")
            }
            return result
        }
    }
    private func queryScalar(_ value: BehaviorJSON) throws -> BehaviorJSON {
        switch value { case .object, .array: throw BehaviorError("QUERY_BINDING"); default: break }; _ = try value.encoded(maximum: 65536); return value
    }
    private func queryBinding(_ condition: BehaviorJSON, arguments: [BehaviorJSON]) throws -> BehaviorJSON {
        let index = try condition.get("argument").integer; guard arguments.indices.contains(index) else { throw BehaviorError("QUERY_ARGUMENT") }; let value = arguments[index]
        guard condition.get("operator") == .string("in") else { return try queryScalar(value) }
        let items = try value.items; guard items.count <= 100 else { throw BehaviorError("QUERY_BINDING") }; for item in items { _ = try queryScalar(item) }; _ = try value.encoded(maximum: 65536); return value
    }
    private func queryCommand(_ metadata: BehaviorJSON, arguments: [BehaviorJSON]) throws -> BehaviorJSON {
        let fields = try Set(metadata.get("fields").items.map { try $0.text }); var conditions: [BehaviorJSON] = []
        for condition in try metadata.get("where").items {
            let field = try condition.get("field").text; let op = try condition.get("operator").text
            guard fields.contains(field), ["eq", "ne", "lt", "lte", "gt", "gte", "in", "isNull", "isNotNull"].contains(op) else { throw BehaviorError("QUERY_BINDING") }
            var bound: [String: BehaviorJSON] = ["field": .string(field), "operator": .string(op)]
            if op != "isNull" && op != "isNotNull" { bound["value"] = try queryBinding(condition, arguments: arguments) }; conditions.append(.object(bound))
        }
        let bindings = BehaviorJSON.array(conditions); _ = try bindings.encoded(maximum: 65536)
        let order = metadata.get("orderBy")
        for item in try order.items { guard fields.contains(try item.get("field").text), ["asc", "desc"].contains(try item.get("direction").text) else { throw BehaviorError("QUERY_ORDER") } }
        let kind = try metadata.get("operation").text; guard ["find", "findOne", "count", "exists", "delete"].contains(kind) else { throw BehaviorError("QUERY_KIND") }
        var query: [String: BehaviorJSON] = ["kind": .string(kind == "findOne" ? "find" : kind), "where": bindings, "orderBy": order]
        if kind == "findOne" { query["page"] = try BehaviorPage.normalize(.object(["size": .number(1)]), policy: .object(["defaultSize": .number(1), "maxSize": .number(1), "sort": metadata.get("fields")])) }
        if kind == "find" {
            var input = BehaviorJSON.object([:])
            if metadata.get("pageArgument") != .null { let index = try metadata.get("pageArgument").integer; guard arguments.indices.contains(index) else { throw BehaviorError("QUERY_ARGUMENT") }; input = arguments[index] }
            let page = try BehaviorPage.normalize(input, policy: .object(["defaultSize": metadata.get("defaultSize"), "maxSize": metadata.get("maxSize"), "sort": metadata.get("fields")]))
            if try !page.get("sort").items.isEmpty { throw BehaviorError("QUERY_ORDER") }; query["page"] = page
        }
        let entity = metadata.get("entity")
        var command: [String: BehaviorJSON] = ["operation": .string("query"), "entity": entity.get("name"), "table": entity.get("table"), "idField": entity.get("id"), "query": .object(query)]
        if let joined = try BehaviorOpenTransactions.open[entity.get("adapter").text] { command["transaction"] = .string(joined) }
        return .object(command)
    }
    fileprivate func executeEntity(_ mapping: BehaviorJSON, operation: String, value: BehaviorJSON) async throws -> BehaviorJSON {
        try await self.operation { execution in
            let adapter = try self.entities(mapping.get("adapter").text)
            var command = try mapping.members; command.removeValue(forKey: "adapter"); command["operation"] = .string(operation); command[operation == "find" ? "page" : operation == "save" ? "value" : "id"] = value
            if let joined = try BehaviorOpenTransactions.open[mapping.get("adapter").text] { command["transaction"] = .string(joined) }
            return try await self.adapter(execution, { try await adapter.execute(.object(command), execution) })
        }
    }
}
struct BehaviorRestClient {
    let context: BehaviorContext
    let name: String
    func get(_ path: String) throws -> BehaviorRestCall { try BehaviorRestCall(context, name, "GET", path) }
    func post(_ path: String) throws -> BehaviorRestCall { try BehaviorRestCall(context, name, "POST", path) }
    func put(_ path: String) throws -> BehaviorRestCall { try BehaviorRestCall(context, name, "PUT", path) }
    func patch(_ path: String) throws -> BehaviorRestCall { try BehaviorRestCall(context, name, "PATCH", path) }
    func delete(_ path: String) throws -> BehaviorRestCall { try BehaviorRestCall(context, name, "DELETE", path) }
    func head(_ path: String) throws -> BehaviorRestCall { try BehaviorRestCall(context, name, "HEAD", path) }
    func options(_ path: String) throws -> BehaviorRestCall { try BehaviorRestCall(context, name, "OPTIONS", path) }
}
struct BehaviorRestCall {
    private let context: BehaviorContext
    private let name: String
    private let method: String
    private let path: String
    private var parameters: [String: BehaviorJSON] = [:]
    private var fields: [String: BehaviorJSON] = [:]
    private var payload: BehaviorJSON?
    init(_ context: BehaviorContext, _ name: String, _ method: String, _ path: String) throws {
        guard BehaviorContext.safePath(path) else { throw BehaviorError("HTTP_PATH") }
        self.context = context; self.name = name; self.method = method; self.path = path
    }
    func query(_ values: [String: BehaviorJSON]) -> BehaviorRestCall {
        var next = self
        for (key, value) in values { next.parameters[key] = value }
        return next
    }
    func header(_ key: String, _ value: String) -> BehaviorRestCall {
        var next = self
        next.fields[key] = .string(value)
        return next
    }
    func body(_ value: BehaviorJSON) throws -> BehaviorRestCall {
        guard !["GET", "HEAD"].contains(method) else { throw BehaviorError("HTTP_BODY") }
        var next = self
        next.payload = value
        return next
    }
    func retrieve() async throws -> BehaviorJSON {
        var descriptor: [String: BehaviorJSON] = ["query": .object(parameters), "headers": .object(fields)]
        if let payload { descriptor["body"] = payload }
        return try await context.restExchange(name, method, path, .object(descriptor))
    }
}
struct BehaviorRepository {
    let context: BehaviorContext; let mapping: BehaviorJSON
    func find(_ page: BehaviorJSON = .object([:])) async throws -> BehaviorJSON { try await context.executeEntity(mapping, operation: "find", value: BehaviorPage.normalize(page, policy: .object(["defaultSize": .number(20), "maxSize": .number(100), "sort": .array([mapping.get("idField")])]))) }
    private func identifier(_ id: BehaviorJSON) throws {
        if case .string(let value) = id { guard !value.isEmpty, value.utf8.count <= 256 else { throw BehaviorError("ENTITY_ID") } } else { _ = try id.integer }
    }
    func save(_ value: BehaviorJSON) async throws -> BehaviorJSON { try identifier(value.get(mapping.get("idField").text)); _ = try value.encoded(); return try await context.executeEntity(mapping, operation: "save", value: value) }
    func delete(_ id: BehaviorJSON) async throws -> BehaviorJSON { try identifier(id); return try await context.executeEntity(mapping, operation: "delete", value: id) }
}

enum BehaviorCalls {
    nonisolated(unsafe) static var keys: [String] = []

    static func record(_ key: String) {
        if !keys.contains(key) { keys.append(key) }
    }

    static func take() -> BehaviorJSON {
        let taken = keys
        keys = []
        return .array(taken.map { .string($0) })
    }
}
