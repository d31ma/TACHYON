using Models;

[RequestRoute("/money")]
[Public]
class MoneyController {
    [Needs]
    readonly MoneyService _money;
    [GetRoute]
    public Money GetMoney() {
        return new Money(_money.amount());
    }
}
