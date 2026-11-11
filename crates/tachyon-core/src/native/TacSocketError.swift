struct TacSocketError: Error, CustomStringConvertible {
    static let known: Set<String> = ["HTTP_BODY", "HTTP_MISSING", "HTTP_REQUEST", "CANCELLED"]
    let code: String
    let message: String

    init(_ code: String?, message: String? = nil) {
        self.message = message ?? "The socket message was not sent."
        if let code, TacSocketError.known.contains(code) {
            self.code = code
        } else {
            self.code = "HTTP_NETWORK"
        }
    }

    var description: String { code + ": " + message }
}
