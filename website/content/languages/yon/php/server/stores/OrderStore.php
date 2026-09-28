<?php
use Models\Order;

#[Store]
class OrderStore
{
    /** @return list<Order> */
    public static function all(): array
    {
        return [new Order('anvil', 4)];
    }
}
