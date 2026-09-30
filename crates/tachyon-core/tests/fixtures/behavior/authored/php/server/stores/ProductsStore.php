<?php
#[Store(ProductEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})]
interface ProductsStore extends Repository {
    public function find_by_status($status): array;
    public function deleteByStatus($status): int;
    public function countByStatus($status): int;
    #[Query({"operation":"exists","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})]
    public function available($status): bool;
}
