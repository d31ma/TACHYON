<?php
use Models\ProductReport;

#[RequestRoute("/products")]
#[Public]
class ProductController {
    #[GetRoute]
    public function getProduct(): ProductReport {
        $signal = new BehaviorSignal();
        $context = Behaviors::create(CatalogSetup::class, ['partition' => 'public-catalog', 'signal' => $signal]);
        $products = Behaviors::construct(ProductStore::class, ['behaviors' => $context]);
        $activeStatus = 'active';
        $active = $products->findByStatus($activeStatus);
        $activeCount = $products->countByStatus($activeStatus);
        $page = $products->bounded(['size' => 5]);
        $others = $products->excluding($activeStatus, $page);
        $context->dispose();
        return new ProductReport($active, $activeCount, $others);
    }
}
