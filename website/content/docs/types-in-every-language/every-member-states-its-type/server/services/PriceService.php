<?php
use Models\Price;

#[Service]
class PriceService
{
    const CURRENCY = 'EUR';

    public function priceFor(int $amount, int $count): Price
    {
        return new Price($amount * $count, self::CURRENCY);
    }
}
