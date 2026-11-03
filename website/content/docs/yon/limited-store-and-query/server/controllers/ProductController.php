<?php
use Models\ProductReport;

#[RequestRoute("/products")]
#[Public]
class ProductController {
    #[Needs]
    private ProductStore $products;

    #[GetRoute]
    public function getProduct(): ProductReport {
        $activeStatus = 'active';
        $active = $this->products->findByStatus($activeStatus);
        $activeCount = $this->products->countByStatus($activeStatus);
        $others = $this->products->excluding($activeStatus, ['size' => 5]);
        return new ProductReport($active, $activeCount, $others);
    }
}
