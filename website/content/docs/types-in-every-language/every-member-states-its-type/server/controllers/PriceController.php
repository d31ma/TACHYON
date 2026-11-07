<?php
use Models\Price;

#[RequestRoute("/price")]
#[Public]
class PriceController
{
    #[Needs]
    private PriceService $priceService;

    #[GetRoute]
    public function getPrice(): Price
    {
        return $this->priceService->priceFor(20, 3);
    }
}
