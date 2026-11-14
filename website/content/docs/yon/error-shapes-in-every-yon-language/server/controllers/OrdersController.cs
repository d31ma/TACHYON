using System;
using System.Collections.Generic;

[RequestRoute("/orders")]
[Allow(typeof(Admin))]
class OrdersController {
    [Needs]
    readonly OrderService _orders;

    [GetRoute]
    public List<string> GetOrders() {
        throw new InvalidOperationException("the order store is down");
    }

    [PostRoute]
    public BehaviorJSON PostOrder([FromBody] BehaviorJSON order) {
        return _orders.accept(order);
    }
}
