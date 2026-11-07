<?php
#[Store(ProductEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})]
interface ProductStore {
    public function findByStatus(string $status): array;
    public function countByStatus(string $status): int;
    #[Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
    public function excluding(string $status, #[PageDefaults] array $page): array;
}
