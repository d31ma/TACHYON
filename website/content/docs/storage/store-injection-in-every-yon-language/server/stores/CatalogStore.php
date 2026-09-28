<?php
#[Store]
class CatalogStore {
  #[Needs]
  private DataSource $dataSource;
  public function find(): mixed { return $this->dataSource->query('SELECT 42 AS amount', []); }
}