<?php
class Ticket {
    public function __construct(
        public string $sku,
        public int $level,
    ) {}
}
