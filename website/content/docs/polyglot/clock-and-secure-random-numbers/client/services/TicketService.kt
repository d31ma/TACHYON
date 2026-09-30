@Service
class TicketService {
    private val clock = Clock()
    private val secureRandom = SecureRandom()
    var ticket: String = ""
    var issuedAt: Double = 0.0

    fun issue() {
        ticket = secureRandom.uuid()
        issuedAt = clock.nowMs()
    }
}
