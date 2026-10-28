import type { TaxAnswer } from '/models/TaxAnswer.ts'

@RequestRoute("/tax")
@Public
class TaxController {
  private static readonly COUNT = 3;

  @Needs
  taxDelegate!: TaxDelegate;

  @GetRoute
  getTax(): TaxAnswer {
    return {
      rate: this.taxDelegate.rate("north"),
      total: this.taxDelegate.total(TaxController.COUNT),
      extra: this.taxDelegate.total(TaxController.COUNT, 4),
    };
  }
}
