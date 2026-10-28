<?php
class TaxAnswer {
    public function __construct(public int $rate, public int $total, public int $extra) {}
}
