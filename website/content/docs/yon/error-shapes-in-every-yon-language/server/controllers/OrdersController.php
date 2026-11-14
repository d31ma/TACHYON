<?php
#[RequestRoute("/orders")]
#[Allow(Admin::class)]
class OrdersController {
    #[Needs]
    public OrderService $orderService;

    #[GetRoute]
    public function getOrders(): array {
        throw new RuntimeException('the order store is down');
    }

    #[PostRoute]
    public function postOrder(#[FromBody] array $order): array {
        return $this->orderService->accept($order);
    }
}
