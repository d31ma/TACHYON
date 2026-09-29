@Service
class TicketService {
    private final Clock clock = new Clock();
    private final SecureRandom secureRandom = new SecureRandom();
    String ticket = "";
    double issuedAt = 0;

    void issue() {
        ticket = secureRandom.uuid();
        issuedAt = clock.nowMs();
    }
}
