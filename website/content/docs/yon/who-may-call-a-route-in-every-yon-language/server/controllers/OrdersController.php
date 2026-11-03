<?php
#[RequestRoute("/orders")]
#[SignedIn]
class OrdersController {
    #[GetRoute]
    #[Public]
    public function getOrders(): array {
        return ['lamp', 'desk'];
    }

    #[PostRoute]
    public function postOrder(): array {
        return ['chair'];
    }

    #[DeleteRoute]
    #[Allow(Admin::class)]
    public function deleteOrders(): array {
        return [];
    }
}
