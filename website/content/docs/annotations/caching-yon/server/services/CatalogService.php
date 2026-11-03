<?php
#[Service]
#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
class CatalogService {
    public BehaviorContext $behaviors;
    public int $reads = 0;

    #[OnStart]
    public function open(): void {
        $signal = new BehaviorSignal();
        $this->behaviors = Behaviors::create(CatalogSetup::class, ['partition' => 'public-catalog', 'signal' => $signal]);
    }

    #[Cached]
    public function find(string $sku): array {
        $this->reads++;
        return $this->entry($sku, $this->reads);
    }

    #[UpdateCache({"method":"find"})]
    public function write(string $sku, int $reads): array { return $this->entry($sku, $reads); }

    #[ClearCache({"method":"find"})]
    public function invalidate(string $sku): array { return $this->entry($sku, $this->reads); }

    private function entry(string $sku, int $reads): array { return ['sku' => $sku, 'reads' => $reads]; }
}
