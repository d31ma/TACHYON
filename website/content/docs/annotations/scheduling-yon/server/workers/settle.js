@Schedule({"fixedDelaySeconds":300})
export class Handler {
  /** @type {LedgerService} */
  @Needs
  ledgerService;

  /** @returns {number} */
  POST() {
    return this.ledgerService.total();
  }
}
