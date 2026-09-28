<?php
use Models\CacheReport;

#[RequestRoute("/catalog")]
#[Public]
class CatalogController {
    #[Needs]
    public CatalogService $catalog;

    #[GetRoute]
    public function getCatalog(): CacheReport {
        $sku = 'anvil';
        $first = $this->catalog->find($sku);
        $repeat = $this->catalog->find($sku);
        $this->catalog->write($sku, 99);
        $written = $this->catalog->find($sku);
        $this->catalog->invalidate($sku);
        $refreshed = $this->catalog->find($sku);
        return new CacheReport($first, $repeat, $written, $refreshed);
    }
}
