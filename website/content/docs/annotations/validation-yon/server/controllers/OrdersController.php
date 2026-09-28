<?php
#[RequestRoute("/orders")]
#[Public]
class OrdersController {
    #[Needs]
    public OrderService $orders;
    #[PostRoute]
    public function postOrders(#[FromBody] array $order): array {
        return $this->orders->accept($order);
    }
}
