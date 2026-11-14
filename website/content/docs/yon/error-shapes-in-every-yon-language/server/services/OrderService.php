<?php

#[Service]
class OrderService {
    public function accept(#[Check(OrderInput::class)] array $order): array { return $order; }
}
