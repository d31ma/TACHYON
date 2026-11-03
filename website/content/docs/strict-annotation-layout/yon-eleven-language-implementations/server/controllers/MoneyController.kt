import models.Money

@RequestRoute("/money")
@Public
class MoneyController {
    @Needs
    val money: MoneyService
    @GetRoute
    fun getMoney(): Money = Money(money.amount())
}
