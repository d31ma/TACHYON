<?php
#[Service]
class CatalogService {
    #[Needs]
    public CatalogCounterHelper $catalogCounterHelper;

    public function total(): int {
        return $this->catalogCounterHelper->count() * 2;
    }
}
