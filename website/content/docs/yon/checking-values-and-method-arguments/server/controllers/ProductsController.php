<?php
use Models\Product;

#[RequestRoute("/products")]
#[Public]
class ProductsController {
    #[Needs]
    public ProductService $productService;

    #[PostRoute]
    public function postProduct(#[FromBody] Product $product): Product {
        $this->productService->accept(get_object_vars($product));
        return $product;
    }
}
