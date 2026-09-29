<?php
#[Store(BillingEntity::class, {"fields":["id"],"defaultSize":20,"maxSize":100})]
interface BillingStore {
  public function existsById(int $id): bool;
}
