<?php
use Models\OrderList;

#[RequestRoute("/orders")]
#[Public]
class OrdersController
{
    #[GetRoute]
    public function getOrders(): OrderList
    {
        return new OrderList(OrderService::list());
    }
}
