<?php
#[Topic]
class InventoryRecord {
    public function __construct(public string $sku, public int $level) {}
}
