<?php
#[RequestRoute("/catalog")]
#[Public]
class CatalogController {
    #[Needs]
    public CatalogClient $catalog;

    #[GetRoute]
    public function getCatalog(): array {
        return [
            'found' => $this->catalog->find("anvil", "full", "acme"),
            'saved' => $this->catalog->save(['sku' => "anvil", 'level' => 3]),
            'replaced' => $this->catalog->replace(5, ['sku' => "anvil", 'level' => 4]),
            'renamed' => $this->catalog->rename(6, ['sku' => "hammer", 'level' => 1]),
            'removed' => $this->catalog->remove(7),
        ];
    }
}
