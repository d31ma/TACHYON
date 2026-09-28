<?php
#[RequestRoute("/catalog")]
#[Public]
class CatalogController {
  #[Needs]
  private CatalogStore $catalog;
  #[GetRoute]
  public function getCatalog(): mixed { return $this->catalog->find(); }
}