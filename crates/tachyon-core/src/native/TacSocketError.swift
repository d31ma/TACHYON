struct TacSocketError: Error, CustomStringConvertible {
    static let known: Set<String> = ["HTTP_BODY", "HTTP_MISSING", "HTTP_REQUEST", "CANCELLED"]
    let code: String

    init(_ code: String?) {
        if let code, TacSocketError.known.contains(code) {
            self.code = code
        } else {
            self.code = "HTTP_NETWORK"
        }
    }

    var description: String { code + ": The socket message was not sent." }
}
