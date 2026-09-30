<?php
#[RequestRoute("/products")]
#[Public]
class ProductsController {
    #[GetRoute]
    public function getProducts(): void {
        http_response_code(200);
        header('Content-Type: text/html; charset=utf-8');
        echo '<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>';
    }
}
