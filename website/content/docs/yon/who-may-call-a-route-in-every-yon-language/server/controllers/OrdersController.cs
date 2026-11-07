using System.Collections.Generic;

[RequestRoute("/orders")]
[SignedIn]
class OrdersController {
    [GetRoute]
    [Public]
    public List<string> GetOrders() {
        return new List<string> { "lamp", "desk" };
    }

    [PostRoute]
    public List<string> PostOrder() {
        return new List<string> { "chair" };
    }

    [DeleteRoute]
    [Allow(typeof(Admin))]
    public List<string> DeleteOrders() {
        return new List<string>();
    }
}
