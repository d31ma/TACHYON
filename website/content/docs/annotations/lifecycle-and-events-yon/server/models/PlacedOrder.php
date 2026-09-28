<?php
class PlacedOrder {
    public function __construct(public string $status, public int $total) {}
}
