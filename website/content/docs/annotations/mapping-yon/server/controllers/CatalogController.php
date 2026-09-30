<?php
use Models\CatalogSummary;

#[RequestRoute("/catalog")]
#[Public]
class CatalogController {
    #[Needs]
    private ProductConverter $converter;

    #[GetRoute]
    public function getCatalog(): CatalogSummary {
        $product = ['detail' => ['id' => 7], 'name' => 'Anvil', 'extra' => 'dropped'];
        $summary = $this->converter->toSummary($product);
        return new CatalogSummary($summary);
    }
}
