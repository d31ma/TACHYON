import models.Bill

@RequestRoute("/billing")
@Public
class BillingController {
  @Needs
  val service: BillingService

  @GetRoute
  fun getBilling(): Bill {
    val total = service.total()
    return Bill(total)
  }
}
