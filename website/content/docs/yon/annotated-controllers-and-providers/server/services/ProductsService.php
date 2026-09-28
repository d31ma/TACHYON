<?php
use Models\Product;

#[Service]
#[Lifetime('transient')]
class ProductsService {
  /** @return list<Product> */
  public function list(): array { return [$this->find(1)]; }
  public function find(int $id): Product { return new Product($id, 'Anvil'); }
}
