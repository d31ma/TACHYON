<?php
use Models\Problem;

#[ErrorHandler]
class OrdersErrorHandler {
    #[HandlesError]
    #[ResponseStatus(404)]
    public function missing(OrderNotFound $error): Problem {
        return new Problem('order-not-found');
    }
}
