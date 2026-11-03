<?php
#[RequestRoute("/catalog")]
#[Public]
class CatalogController {
    #[Needs]
    public CatalogService $catalog;
    #[GetRoute]
    public function getCatalog(): array { return $this->catalog->products(); }
}
