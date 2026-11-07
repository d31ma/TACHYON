using Models;

[RequestRoute("/price")]
[Public]
class PriceController {
    [Needs]
    readonly PriceService _priceService;

    [GetRoute]
    public Price GetPrice() {
        return _priceService.PriceFor(20, 3);
    }
}
