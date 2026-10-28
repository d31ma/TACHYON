<?php
#[Service]
class CatalogService {
    #[Needs]
    public CatalogCounterDelegate $catalogCounterDelegate;

    public function total(): int {
        return $this->catalogCounterDelegate->count() * 2;
    }
}
