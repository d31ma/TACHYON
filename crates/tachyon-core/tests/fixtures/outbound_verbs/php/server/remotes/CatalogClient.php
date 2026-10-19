<?php
#[HTTPClient("/api")]
interface CatalogClient {
    #[GetRequest("/products/{id}")]
    public function find(#[PathVariable("id")] string $id, #[RequestParam("view")] string $view, #[RequestHeader("X-Tenant")] string $tenant): array;

    #[PostRequest("/products")]
    public function save(#[RequestBody] array $product): array;

    #[PutRequest("/products/{id}")]
    public function replace(#[PathVariable("id")] int $id, #[RequestBody] array $product): array;

    #[PatchRequest("/products/{id}")]
    public function rename(#[PathVariable("id")] int $id, #[RequestBody] array $product): array;

    #[DeleteRequest("/products/{id}")]
    public function remove(#[PathVariable("id")] int $id): array;
}
