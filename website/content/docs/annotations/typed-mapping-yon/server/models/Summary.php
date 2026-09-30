<?php
use Models\ShortDetail;
class Summary {
public function __construct(public int $id, public string $name, public ShortDetail $detail,
/** @var list<ShortDetail> */
public array $items, public string $kind) {}
}
