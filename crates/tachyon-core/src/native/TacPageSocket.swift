func tacSocketSend<Message: Encodable>(_ client: String, _ message: Message) async throws {
    let text = tacEncode(tacModelValue(message))
    guard text.utf8.count <= 65536 else { throw TacSocketError("HTTP_BODY") }
    guard let owner = TacNativeState.executing, !owner.disposed else { throw TacSocketError("HTTP_REQUEST") }
    guard let emit = TacBridge.emit else { throw TacSocketError("HTTP_MISSING") }
    emit("{\"route\":" + tacEncode(owner.route) + ",\"service\":" + tacEncode(owner.service) + ",\"owner\":" + tacEncode(owner.owner) + ",\"send\":" + tacEncode(client) + ",\"value\":" + text + "}")
}
