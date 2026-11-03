<?php
class Order {
    public function __construct(
        public string $sku,
        public int $level,
    ) {}
}
