<?php
use Models\Problem;

#[RequestRoute("/orders")]
#[Public]
class OrdersController {
    #[GetRoute]
    public function getOrder(): Problem {
        throw new OrderNotFound('order 42');
    }
}
