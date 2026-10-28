import type { TaxAnswer } from '/models/TaxAnswer.ts'

@RequestRoute("/tax")
@Public
class TaxController {
  @Needs
  taxDelegate!: TaxDelegate;

  @GetRoute
  getTax(): TaxAnswer {
    return {
      rate: this.taxDelegate.rate("north"),
      total: this.taxDelegate.total(3),
      extra: this.taxDelegate.total(3, 4),
    };
  }
}
