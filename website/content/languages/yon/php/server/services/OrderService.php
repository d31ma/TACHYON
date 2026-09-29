<?php
use Models\Order;

#[Service]
class OrderService
{
    /** @return list<Order> */
    public static function list(): array
    {
        return [new Order('anvil', 4)];
    }
}
