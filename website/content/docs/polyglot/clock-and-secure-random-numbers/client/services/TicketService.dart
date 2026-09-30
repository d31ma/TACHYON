@Service
class TicketService {
  @Needs()
  final Clock clock;
  @Needs()
  final SecureRandom secureRandom;
  String ticket = '';
  double issuedAt = 0;

  void issue() {
    ticket = secureRandom.uuid();
    issuedAt = clock.nowMs();
  }
}
