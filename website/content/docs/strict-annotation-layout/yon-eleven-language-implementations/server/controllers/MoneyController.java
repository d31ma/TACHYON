import models.Money;

@RequestRoute("/money")
@Public
class MoneyController {
    @Needs
    final MoneyService money;
    @GetRoute
    Money getMoney() {
        return new Money(money.amount());
    }
}
