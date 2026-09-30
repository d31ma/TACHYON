<?php
use Models\Product;
use Models\Detail;
use Models\Summary;
#[RequestRoute("/catalog")]
#[Public]
class CatalogController {
#[Needs]
private ProductConverter $converter;
#[GetRoute]
public function getCatalog(): Summary {
$detail = new Detail("detail", "detail-private");
return $this->converter->map(new Product(7, "Anvil", $detail, [$detail], "product-private"));
}
}
