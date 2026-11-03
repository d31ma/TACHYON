@Service
export default class TicketService {
  @Needs
  clock!: Clock;
  @Needs
  secureRandom!: SecureRandom;
  ticket: string = '';
  issuedAt: number = 0;

  issue(): void {
    this.ticket = this.secureRandom.uuid();
    this.issuedAt = this.clock.now();
  }
}
