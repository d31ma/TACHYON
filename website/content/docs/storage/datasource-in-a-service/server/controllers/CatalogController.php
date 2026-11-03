<?php
#[RequestRoute("/catalog")]
#[Public]
class CatalogController {
  #[Needs]
  private CatalogService $catalog;
  #[GetRoute]
  public function getCatalog(): mixed { return $this->catalog->find(); }
}