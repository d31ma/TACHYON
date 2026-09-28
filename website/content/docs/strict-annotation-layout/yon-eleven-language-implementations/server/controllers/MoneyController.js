/** @import { Money } from '/models/Money.ts' */
@RequestRoute("/money")
@Public
class MoneyController {
  /** @type {MoneyService} */
  @Needs
  moneyService;
  /** @returns {Money} */
  @GetRoute
  getMoney() { return { amount: this.moneyService.amount() }; }
}
