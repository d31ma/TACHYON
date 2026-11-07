<?php
use Models\Product;

#[RequestRoute("/products")]
#[Public]
class ProductsController {
  #[Needs]
  #[Named('ProductsService')]
  private ProductsService $catalog;

  /** @return list<Product> */
  #[GetRoute]
  public function getProducts(): array { return $this->catalog->list(); }

  #[GetRoute("/{id}")]
  public function getProduct(#[FromPath] int $id): Product { return $this->catalog->find($id); }
}
