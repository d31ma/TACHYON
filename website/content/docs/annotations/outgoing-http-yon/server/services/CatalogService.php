<?php
#[Service]
class CatalogService {
    #[Needs]
    public CatalogClient $catalog;
    public function products(): array { return $this->catalog->all(1); }
}
