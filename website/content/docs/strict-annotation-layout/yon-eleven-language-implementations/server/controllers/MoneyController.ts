import type { Money } from '/models/Money.ts'

@RequestRoute("/money")
@Public
class MoneyController {
  @Needs
  readonly money!: MoneyService;
  @GetRoute
  getMoney(): Money { return { amount: this.money.amount() }; }
}
