import type { Bill } from '/models/Bill.ts'

@RequestRoute("/billing")
@Public
class BillingController {
  @Needs
  service!: BillingService;

  @GetRoute
  getBilling(): Bill {
    const total = this.service.total();
    return { total };
  }
}
