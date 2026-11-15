
enum AppleCryptoRefusal: String, Error {
    case unsupported
    case invalid
    case unknown
    case exists
    case rejected
    case locked
    case failed
}
