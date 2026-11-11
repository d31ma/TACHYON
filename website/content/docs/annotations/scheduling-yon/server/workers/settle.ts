@Schedule({"fixedDelaySeconds":300})
export class Handler {
  @Needs
  ledgerService!: LedgerService;

  POST(): number {
    return this.ledgerService.total();
  }
}
