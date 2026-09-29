<?php
#[Store(EntryEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})]
interface EntryStore {
    public function findByStatus(string $status): array;
    public function countByStatus(string $status): int;
}
