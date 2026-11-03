@Service
export default class TicketService {
  /** @type {Clock} */
  @Needs
  clock;
  /** @type {SecureRandom} */
  @Needs
  secureRandom;
  ticket = '';
  issuedAt = 0;

  issue() {
    this.ticket = this.secureRandom.uuid();
    this.issuedAt = this.clock.now();
  }
}
