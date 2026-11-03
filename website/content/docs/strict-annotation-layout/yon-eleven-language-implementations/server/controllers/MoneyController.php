<?php
use Models\Money;

#[RequestRoute("/money")]
#[Public]
class MoneyController {
    #[Needs]
    public MoneyService $money;
    #[GetRoute]
    public function getMoney(): Money {
        return new Money($this->money->amount());
    }
}
