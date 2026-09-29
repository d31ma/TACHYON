@Service
final class TicketService {
    private let clock = Clock()
    private let secureRandom = SecureRandom()
    var ticket: String = ""
    var issuedAt: Double = 0

    func issue() throws {
        ticket = try secureRandom.uuid()
        issuedAt = clock.nowMs()
    }
}
