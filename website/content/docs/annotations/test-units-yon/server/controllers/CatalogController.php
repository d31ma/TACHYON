<?php
#[RequestRoute("/api/catalog")]
#[Public]
class CatalogController {
    #[Needs]
    public CatalogService $catalogService;

    #[GetRoute]
    public function getTotal(): int {
        return $this->catalogService->total();
    }
}
