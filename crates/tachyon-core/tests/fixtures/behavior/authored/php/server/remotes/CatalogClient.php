<?php
#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
#[HTTPClient({"adapter": "api"})]
interface CatalogClient {
    #[Cached]
    #[GetRequest("/products")]
    public function remote(): array;
    #[GetRequest("/products")]
    public function search(#[RequestParam("page")] $page): array;
    #[GetRequest("/products/{id}")]
    public function find(#[PathVariable] $id): array;
}
