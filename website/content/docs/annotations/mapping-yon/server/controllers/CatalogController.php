<?php
use Models\CatalogSummary;

#[RequestRoute("/catalog")]
#[Public]
class CatalogController {
    #[GetRoute]
    public function getCatalog(): CatalogSummary {
        $signal = new BehaviorSignal();
        $options = ['partition' => 'public-catalog', 'signal' => $signal];
        $context = Behaviors::create(CatalogSetup::class, $options);
        $mapper = Behaviors::construct(ProductConverter::class, ['behaviors' => $context]);
        $product = ['detail' => ['id' => 7], 'name' => 'Anvil', 'extra' => 'dropped'];
        $summary = $mapper->toSummary($product);
        $context->dispose();
        return new CatalogSummary($summary);
    }
}
