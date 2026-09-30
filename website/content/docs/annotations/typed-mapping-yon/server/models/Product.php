<?php
use Models\Detail;
class Product {
public function __construct(public int $id, public string $name, public Detail $detail,
/** @var list<Detail> */
public array $items, public string $secret) {}
}
