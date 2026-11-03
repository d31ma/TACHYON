<?php
use Models\Product;

#[RequestRoute("/products")]
#[Public]
class ProductsController {
    /** @return list<Product> */
    #[GetRoute]
    public function getProducts(): array {
        return [new Product('anvil')];
    }

    #[PostRoute]
    #[ResponseStatus(201)]
    public function postProduct(#[FromBody] Product $product): Product {
        return $product;
    }
}
