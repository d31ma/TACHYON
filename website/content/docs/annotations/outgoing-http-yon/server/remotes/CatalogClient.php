<?php
#[HTTPClient]
interface CatalogClient {
    #[GetRequest("/products")]
    public function all(#[RequestParam("page")] int $page): array;

    #[PostRequest("/products")]
    public function add(#[RequestBody] array $product): array;
}
