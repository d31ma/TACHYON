enum TacMember {
    case field(() -> Any?, ((Any?) -> Void)?)
    case method(([Any?]) throws -> Any?)
    case listener(TacListener)
    case hook(Bool, () throws -> Void)

    static func field(_ read: @escaping () -> Any?) -> TacMember { .field(read, nil) }
}

final class TacListener {
    let events: [String]
    let replay: Bool
    let order: Int
    let condition: Any?
    let models: Any?
    let returns: String?
    let asynchronous: Bool
    let invoke: (Any?) throws -> Any?

    init(
        _ events: [String], _ replay: Bool, _ order: Int, _ condition: String?,
        _ models: String?, _ returns: String?,
        _ asynchronous: Bool, _ invoke: @escaping (Any?) throws -> Any?
    ) {
        self.events = events
        self.replay = replay
        self.order = order
        self.condition = condition.flatMap(tacParse)
        self.models = models.flatMap(tacParse)
        self.returns = returns
        self.asynchronous = asynchronous
        self.invoke = invoke
    }
}

private func tacParse(_ text: String) -> Any? {
    tacParseJSON(text)
}

func tacArgument<T: Decodable>(_ values: [Any?], _ index: Int) throws -> T {
    try tacModel(T.self, index < values.count ? values[index] : nil)
}

func tacArgument<T>(_ values: [Any?], _ index: Int) throws -> T {
    guard index < values.count, let value = values[index] as? T else { throw TacWrongArgument.value }
    return value
}

func tacField<T: Decodable>(_ type: T.Type, _ value: Any?) -> T? {
    try? tacModel(T.self, value)
}

func tacField<T>(_ type: T.Type, _ value: Any?) -> T? {
    value as? T
}

enum TacWrongArgument: Error { case value }

public final class ApplicationEventPublisher {
    public init() {}

    public func publishEvent<T: Encodable>(_ value: T, _ retain: Bool = false) {
        guard let object = tacModelValue(value) as? [String: Any?] else { tacEventFailure(); return }
        tacEventPublish(String(describing: type(of: value)), object, retain)
    }
}

private func tacEncode(_ text: String) -> String {
    var encoded = "\""
    for character in text.unicodeScalars {
        switch character {
        case "\"": encoded += "\\\""
        case "\\": encoded += "\\\\"
        case "\n": encoded += "\\n"
        case "\t": encoded += "\\t"
        case "\r": encoded += "\\r"
        default:
            if character.value < 0x20 {
                var hexadecimal = String(character.value, radix: 16)
                while hexadecimal.count < 4 { hexadecimal = "0" + hexadecimal }
                encoded += "\\u" + hexadecimal
            } else {
                encoded.unicodeScalars.append(character)
            }
        }
    }
    return encoded + "\""
}

private func tacEncode(_ value: Any?) -> String {
    switch value {
    case nil: return "null"
    case is Void: return "null"
    case let value as Bool: return value ? "true" : "false"
    case let value as Int: return String(value)
    case let value as Double:
        guard value.isFinite else { return "null" }
        return value == value.rounded() && value.magnitude < 9_007_199_254_740_992
            ? String(Int64(value)) : String(value)
    case let value as String: return tacEncode(value)
    case let values as [Any?]: return "[" + values.map(tacEncode).joined(separator: ",") + "]"
    case let values as [String: Any?]:
        return "{"
            + values.map { tacEncode($0.key) + ":" + tacEncode($0.value) }.joined(separator: ",")
            + "}"
    case let value: return tacEncode(String(describing: value!))
    }
}

private func tacAssignProperties(_ activation: TacActivation, _ props: [String: Any]?) {
    guard let props else { return }
    let required = tacServiceRequired(activation.service)
    for (property, given) in props where required.contains("*") || required.contains(property) {
        if case .field(_, let write?)? = activation.members[property] { write(tachyonSwiftJSONValue(given)) }
    }
}

private func tacHandle(_ raw: String) throws -> String {
    guard let request = tachyonParseJSONRequest(raw) else {
        return "{\"error\":\"Malformed companion request.\"}"
    }
    guard let operation = request["op"] as? String else {
        return "{\"error\":\"Companion request has no operation.\"}"
    }
    if operation == "host-dispose" { tacDisposeAll(); return "{\"value\":null}" }
    guard let route = request["route"] as? String, let owner = request["owner"] as? String,
          tacValidOwner(owner) else { return "{\"error\":\"Missing native owner identity.\",\"code\":\"TY_NATIVE_OWNER\"}" }
    guard let service = request["service"] as? String else {
        return "{\"error\":\"Missing companion name.\",\"code\":\"TY_NATIVE_SERVICE\"}"
    }
    let existing = TacNativeState.activations[owner]
    if operation == "init" && (existing == nil || existing?.route != route || existing?.service != service) {
        if existing != nil { tacDisposeOwner(owner) }
        guard tacServiceKnown(service) else {
            return "{\"error\":\"Unknown native service.\",\"code\":\"TY_NATIVE_SERVICE\"}"
        }
        guard let members = tacServiceMembers(service, request["props"] as? [String: Any] ?? [:]) else {
            return "{\"error\":\"Native service is missing a required property or it has the wrong type.\",\"code\":\"TY_NATIVE_REQUIRED\"}"
        }
        if TacNativeState.activations.count >= 256 { return "{\"error\":\"Too many native activations.\"}" }
        let activation = TacActivation(route, service, owner, members)
        TacNativeState.activations[owner] = activation
        TacNativeState.executing = activation
        do {
            tacAssignProperties(activation, request["props"] as? [String: Any])
            for (_, member) in activation.members.sorted(by: { $0.key < $1.key }) {
                if case .hook(false, let invoke) = member { try invoke() }
            }
            tacDrain(activation)
        } catch { tacDisposeOwner(owner); throw error }
    }
    guard let activation = TacNativeState.activations[owner], activation.route == route, activation.service == service else {
        return "{\"error\":\"Stale native owner.\",\"code\":\"TY_NATIVE_OWNER\"}"
    }
    if operation == "dispose" { tacDisposeOwner(owner); return "{\"value\":null}" }
    let tac = activation.members
    TacNativeState.executing = activation
    if operation == "init" {
        var fields: [Any?] = []
        var methods: [Any?] = []
        for (name, member) in tac {
            if case .field = member { fields.append(name) }
            if case .method = member { methods.append(name) }
        }
        return "{\"value\":{\"fields\":" + tacEncode(fields) + ",\"methods\":"
            + tacEncode(methods) + "}}"
    }
    guard let name = request["name"] as? String, let member = tac[name] else {
        return "{\"error\":\"Unknown companion member.\"}"
    }
    switch (operation, member) {
    case ("get", .field(let read, _)):
        return "{\"value\":" + tacEncode(read()) + "}"
    case ("set", .field(_, let write)):
        guard let write else {
            return "{\"error\":" + tacEncode("Companion field is read-only: " + name) + "}"
        }
        write(tachyonSwiftJSONValue(request["value"]))
        return "{\"value\":null}"
    case ("call", .method(let invoke)):
        let arguments: [Any?]
        if let values = request["args"] as? [Any] {
            arguments = values.map(tachyonSwiftJSONValue)
        } else {
            arguments = []
        }
        return "{\"value\":" + tacEncode(try invoke(arguments)) + "}"
    default:
        return "{\"error\":" + tacEncode("Companion member does not support " + operation + ": " + name) + "}"
    }
}

func tacNativeInvoke(_ request: String) -> String {
    guard tacOnOwnerThread(), !TacNativeState.handling else { return "{\"error\":\"Native owner is busy or on another thread.\"}" }
    TacNativeState.handling = true
    defer { if let owner = TacNativeState.executing { tacDrain(owner) }; TacNativeState.executing = nil; TacNativeState.handling = false }
    do { return try tacHandle(request) }
    catch { return tacFailureAnswer(error) }
}

public enum TacBridge {
    nonisolated(unsafe) public static var emit: ((String) -> Void)?
}

private final class TacActivation {
    let route: String
    let service: String
    let owner: String
    let members: [String: TacMember]
    let listeners: [TacListener]
    var disposed = false
    var queue: [(String, String, Bool)] = []
    var retained: [String: String] = [:]
    var deferred: [(TacListener, String)] = []
    init(_ route: String, _ service: String, _ owner: String, _ members: [String: TacMember]) {
        self.route = route; self.service = service; self.owner = owner; self.members = members
        var declared: [(Int, TacListener)] = []
        for (position, entry) in members.sorted(by: { $0.key < $1.key }).enumerated() {
            if case .listener(let listener) = entry.value { declared.append((position, listener)) }
        }
        declared.sort { left, right in
            left.1.order == right.1.order ? left.0 < right.0 : left.1.order < right.1.order
        }
        self.listeners = declared.map { $0.1 }
    }
}
private enum TacNativeState {
    nonisolated(unsafe) static var activations: [String: TacActivation] = [:]
    nonisolated(unsafe) static var executing: TacActivation?
    nonisolated(unsafe) static var handling = false
}

private func tacEventValid(_ value: Any?, _ depth: Int = 0) -> Bool {
    if depth > 64 { return false }
    switch value {
    case nil, is Void, is String, is Bool, is Int: return true
    case let number as Double: return number.isFinite
    case let list as [Any?]: return list.allSatisfy { tacEventValid($0, depth + 1) }
    case let map as [String: Any?]: return map.values.allSatisfy { tacEventValid($0, depth + 1) }
    default: return false
    }
}

private func tacEventPublish(_ name: String, _ value: Any?, _ retain: Bool) {
    let json = tacEncode(value)
    guard tacOnOwnerThread(), let owner = TacNativeState.executing, !owner.disposed, TacNativeState.activations[owner.owner] === owner,
          owner.queue.count < 128, tacEventValid(value),
          json.utf8.count <= 65536 else { tacEventFailure(); return }
    owner.queue.append((name, json, retain))
}

private enum TacAbsent { case value }

private func tacAbsent(_ value: Any?) -> Bool { value is TacAbsent }

private func tacSegmentKey(_ segment: Any?) -> String? {
    switch segment {
    case let key as String: return key
    case let index as Int: return String(index)
    default: return nil
    }
}

private func tacSegmentIndex(_ segment: Any?) -> Int? {
    switch segment {
    case let index as Int: return index
    case let key as String: return Int(key)
    default: return nil
    }
}

private func tacOperand(_ node: [String: Any?], _ value: Any?, _ event: String) -> Any? {
    switch node["k"] as? String {
    case "lit": return node["v"] ?? nil
    case "event": return event
    default:
        var current: Any? = value
        for segment in (node["path"] as? [Any?] ?? []) {
            if let map = current as? [String: Any?] {
                guard let key = tacSegmentKey(segment), let held = map[key] else { return TacAbsent.value }
                current = held
            } else if let list = current as? [Any?] {
                guard let index = tacSegmentIndex(segment), index >= 0, index < list.count else {
                    return TacAbsent.value
                }
                current = list[index]
            } else {
                return TacAbsent.value
            }
        }
        return current
    }
}

private func tacNumber(_ value: Any?) -> Double? {
    if value is Bool { return nil }
    if let number = value as? Int { return Double(number) }
    if let number = value as? Double { return number }
    return nil
}

private func tacTruthy(_ value: Any?) -> Bool {
    switch value {
    case nil, is TacAbsent: return false
    case let flag as Bool: return flag
    case let text as String: return !text.isEmpty
    default:
        guard let number = tacNumber(value) else { return true }
        return number != 0 && !number.isNaN
    }
}

private func tacIdentical(_ left: Any?, _ right: Any?) -> Bool {
    if tacAbsent(left) || tacAbsent(right) { return tacAbsent(left) && tacAbsent(right) }
    let leftNull = left == nil
    let rightNull = right == nil
    if leftNull || rightNull { return leftNull && rightNull }
    if let left = left as? Bool { return (right as? Bool) == left }
    if let left = left as? String { return (right as? String) == left }
    if let left = tacNumber(left), let right = tacNumber(right) { return left == right }
    if let left = left as? [Any?], let right = right as? [Any?] {
        return left.count == right.count
            && !zip(left, right).contains { !tacIdentical($0, $1) }
    }
    if let left = left as? [String: Any?], let right = right as? [String: Any?] {
        return left.count == right.count
            && !left.contains { key, value in
                guard let held = right[key] else { return true }
                return !tacIdentical(value, held)
            }
    }
    return false
}

private func tacOrders(_ operation: String, _ left: Any?, _ right: Any?) -> Bool {
    if let left = tacNumber(left), let right = tacNumber(right) {
        switch operation {
        case "lt": return left < right
        case "le": return left <= right
        case "gt": return left > right
        default: return left >= right
        }
    }
    guard let left = left as? String, let right = right as? String else { return false }
    switch operation {
    case "lt": return left < right
    case "le": return left <= right
    case "gt": return left > right
    default: return left >= right
    }
}

private func tacMatches(_ matcher: Any?, _ value: Any?, _ event: String) -> Bool {
    guard let node = matcher as? [String: Any?], let kind = node["k"] as? String else { return false }
    switch kind {
    case "and": return tacMatches(node["l"] ?? nil, value, event) && tacMatches(node["r"] ?? nil, value, event)
    case "or": return tacMatches(node["l"] ?? nil, value, event) || tacMatches(node["r"] ?? nil, value, event)
    case "not": return !tacMatches(node["e"] ?? nil, value, event)
    case "truthy":
        guard let operand = (node["e"] ?? nil) as? [String: Any?] else { return false }
        return tacTruthy(tacOperand(operand, value, event))
    case "cmp":
        guard let left = (node["l"] ?? nil) as? [String: Any?],
              let right = (node["r"] ?? nil) as? [String: Any?],
              let operation = node["op"] as? String else { return false }
        let leftValue = tacOperand(left, value, event)
        let rightValue = tacOperand(right, value, event)
        if operation == "eq" { return tacIdentical(leftValue, rightValue) }
        if operation == "ne" { return !tacIdentical(leftValue, rightValue) }
        return tacOrders(operation, leftValue, rightValue)
    default: return false
    }
}

private func tacRepublish(_ result: Any?, _ returns: String?) {
    guard let returns else { return }
    let items = (result as? [Any?]) ?? [result]
    for item in items.prefix(32) {
        guard let item, let model = item as? any Encodable, let value = tacModelValue(model) else { continue }
        tacEventPublish(returns, value, false)
    }
}

private func tacFits(_ models: Any?, _ shape: String, _ value: Any?) -> Bool {
    if shape.hasPrefix("?") { return value == nil || tacFits(models, String(shape.dropFirst()), value) }
    if shape.hasPrefix("[]") {
        guard let items = value as? [Any?] else { return false }
        return items.allSatisfy { tacFits(models, String(shape.dropFirst(2)), $0) }
    }
    switch shape {
    case "any": return true
    case "string": return value is String
    case "integer": return value is Int
    case "number": return value is Int || value is Double
    case "boolean": return value is Bool
    default: break
    }
    guard let declared = models as? [String: Any?], let model = declared[shape] as? [String: Any?],
          let fields = model["fields"] as? [String: Any?], let object = value as? [String: Any?]
    else { return false }
    if (model["open"] as? Bool) != true && object.keys.contains(where: { fields[$0] == nil }) { return false }
    for (name, declaredShape) in fields {
        guard let field = declaredShape as? String else { return false }
        let required = field.hasPrefix("!")
        guard let found = object[name] else {
            if required { return false }
            continue
        }
        if !tacFits(models, required ? String(field.dropFirst()) : field, found) { return false }
    }
    return true
}

private func tacDeliver(_ listener: TacListener, _ json: String, _ republish: Bool) {
    do {
        let result = try listener.invoke(tacParseJSON(json))
        if republish { tacRepublish(result, listener.returns) }
    } catch { tacEventFailure() }
}

private func tacDrain(_ owner: TacActivation) {
    var delivered = 0
    while !owner.disposed {
        if !owner.deferred.isEmpty {
            for (listener, json) in owner.deferred { tacDeliver(listener, json, false) }
            owner.deferred.removeAll()
            continue
        }
        if owner.queue.isEmpty { return }
        if delivered == 128 { owner.queue.removeAll(); tacEventFailure(); return }
        delivered += 1
        let (name, json, retain) = owner.queue.removeFirst()
        if retain { if owner.retained[name] == nil && owner.retained.count == 32 { tacEventFailure(); continue }; owner.retained[name] = json }
        let value = tacParseJSON(json)
        for listener in owner.listeners where listener.events.contains(name) {
            if let models = listener.models, !tacFits(models, name, value) { tacEventFailure(); continue }
            if let condition = listener.condition, !tacMatches(condition, value, name) { continue }
            if listener.asynchronous {
                if owner.deferred.count >= 128 { tacEventFailure(); continue }
                owner.deferred.append((listener, json))
                continue
            }
            tacDeliver(listener, json, true)
        }
        TacBridge.emit?("{\"route\":" + tacEncode(owner.route) + ",\"service\":" + tacEncode(owner.service) + ",\"owner\":" + tacEncode(owner.owner) + ",\"name\":" + tacEncode(name) + ",\"value\":" + json + ",\"retain\":" + String(retain) + "}")
    }
}

private func tacDisposeActivation(_ owner: TacActivation) {
    owner.disposed = true
    owner.queue.removeAll(); owner.retained.removeAll(); owner.deferred.removeAll()
    for (_, member) in owner.members.sorted(by: { $0.key < $1.key }) {
        if case .hook(true, let invoke) = member { do { try invoke() } catch { tacEventFailure() } }
    }
}
private func tacDisposeOwner(_ id: String) {
    guard let owner = TacNativeState.activations.removeValue(forKey: id) else { return }
    if TacNativeState.executing === owner { TacNativeState.executing = nil }
    tacDisposeActivation(owner)
}
private func tacDisposeAll() {
    let owners = TacNativeState.activations
    TacNativeState.activations = [:]
    TacNativeState.executing = nil
    for (_, owner) in owners { tacDisposeActivation(owner) }
}
